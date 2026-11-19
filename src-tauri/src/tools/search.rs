use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use encoding_rs::{GB18030, UTF_16BE, UTF_16LE};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::RegexBuilder;
use walkdir::WalkDir;

use crate::event::{AppEvent, EventBus, emit};
use crate::tools::event::ToolEventDto;
use crate::tools::model::{SearchDoneDto, SearchHitDto, SearchInput, ToolErrorDto};
use crate::tools::tasks::ToolTasks;

const DEFAULT_EXCLUDES: [&str; 4] = [".git/**", "node_modules/**", "target/**", "dist/**"];
const HIT_BATCH: usize = 25;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);

pub async fn run(
    task_id: String,
    input: SearchInput,
    cancel: Arc<AtomicBool>,
    tasks: ToolTasks,
    bus: EventBus,
) {
    let run_id = task_id.clone();
    let run_bus = bus.clone();
    let result =
        tokio::task::spawn_blocking(move || search_blocking(&run_id, &input, &cancel, &run_bus))
            .await;
    tasks.finish(&task_id);
    let outcome = match result {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(error)) => SearchOutcome {
            files: 0,
            matches: 0,
            skipped: 0,
            truncated: false,
            error: Some(error),
        },
        Err(error) => SearchOutcome {
            files: 0,
            matches: 0,
            skipped: 0,
            truncated: false,
            error: Some(ToolErrorDto::new("searchFailed", error.to_string())),
        },
    };
    emit(
        &bus,
        AppEvent::Tool(ToolEventDto::SearchDone(SearchDoneDto {
            task_id,
            files: outcome.files,
            matches: outcome.matches,
            skipped: outcome.skipped,
            truncated: outcome.truncated,
            error: outcome.error,
        })),
    );
}

struct SearchOutcome {
    files: u64,
    matches: u64,
    skipped: u64,
    truncated: bool,
    error: Option<ToolErrorDto>,
}

fn search_blocking(
    task_id: &str,
    input: &SearchInput,
    cancel: &AtomicBool,
    bus: &EventBus,
) -> Result<SearchOutcome, ToolErrorDto> {
    let folder = PathBuf::from(input.folder.trim());
    if !folder.is_dir() {
        return Err(ToolErrorDto::new(
            "invalidFolder",
            "Select an existing folder",
        ));
    }
    let query = input.query.trim();
    if query.is_empty() {
        return Err(ToolErrorDto::new("noQuery", "Enter search text"));
    }
    let pattern = if input.mode == "regex" {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(!input.case_sensitive)
        .multi_line(true)
        .build()
        .map_err(|error| ToolErrorDto::new("invalidRegex", error.to_string()))?;
    let include = build_globset(&input.include, &[])?;
    let exclude = build_globset(&input.exclude, &DEFAULT_EXCLUDES)?;
    let max_file_size = input.max_file_size.clamp(1024, 1024 * 1024 * 1024);
    let max_results = input.max_results.clamp(1, 10_000);
    let mut files = 0u64;
    let mut matches = 0u64;
    let mut skipped = 0u64;
    let mut truncated = false;
    let mut batch = Vec::new();
    let mut last_progress = Instant::now();

    for entry in WalkDir::new(&folder)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if !input.skip_hidden {
                return true;
            }
            entry
                .path()
                .strip_prefix(&folder)
                .map(|path| {
                    !path.components().any(|component| {
                        let name = component.as_os_str().to_string_lossy();
                        name.starts_with('.') && name != "." && name != ".."
                    })
                })
                .unwrap_or(true)
        })
    {
        if cancel.load(Ordering::Relaxed) {
            return Err(ToolErrorDto::new("cancelled", "Search cancelled"));
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(&folder)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if (include.is_some() && !include.as_ref().is_some_and(|set| set.is_match(&relative)))
            || exclude.as_ref().is_some_and(|set| set.is_match(&relative))
        {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if metadata.len() > max_file_size {
            skipped += 1;
            continue;
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if is_binary(&bytes, &input.encoding) {
            skipped += 1;
            continue;
        }
        let Some(text) = decode_bytes(&bytes, &input.encoding) else {
            skipped += 1;
            continue;
        };
        files += 1;
        let display_path = path.to_string_lossy().into_owned();
        'lines: for (line_index, line) in text.lines().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err(ToolErrorDto::new("cancelled", "Search cancelled"));
            }
            for found in regex.find_iter(line) {
                matches += 1;
                batch.push(SearchHitDto {
                    path: display_path.clone(),
                    line: (line_index + 1) as u32,
                    column: (line[..found.start()].chars().count() + 1) as u32,
                    preview: preview(line, found.start(), found.end()),
                    matched: found.as_str().to_owned(),
                });
                if matches as usize >= max_results {
                    truncated = true;
                    break 'lines;
                }
            }
        }
        if batch.len() >= HIT_BATCH {
            emit_hits(bus, task_id, &mut batch);
        }
        if last_progress.elapsed() >= PROGRESS_INTERVAL {
            emit_progress(bus, task_id, files, matches);
            last_progress = Instant::now();
        }
        if truncated {
            break;
        }
    }

    if !batch.is_empty() {
        emit_hits(bus, task_id, &mut batch);
    }
    emit_progress(bus, task_id, files, matches);
    Ok(SearchOutcome {
        files,
        matches,
        skipped,
        truncated,
        error: None,
    })
}

fn build_globset(value: &str, defaults: &[&str]) -> Result<Option<GlobSet>, ToolErrorDto> {
    let mut builder = GlobSetBuilder::new();
    let mut count = 0;
    for pattern in value
        .split([',', '\n'])
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .chain(defaults.iter().copied())
    {
        builder.add(
            Glob::new(pattern)
                .map_err(|error| ToolErrorDto::new("invalidGlob", error.to_string()))?,
        );
        count += 1;
    }
    if count == 0 {
        return Ok(None);
    }
    builder
        .build()
        .map(Some)
        .map_err(|error| ToolErrorDto::new("invalidGlob", error.to_string()))
}

fn decode_bytes<'a>(bytes: &'a [u8], encoding: &str) -> Option<Cow<'a, str>> {
    let encoding = encoding.to_ascii_lowercase();
    if encoding == "utf-8" || encoding == "auto" && std::str::from_utf8(bytes).is_ok() {
        return Some(String::from_utf8_lossy(bytes));
    }
    if bytes.starts_with(&[0xff, 0xfe]) && matches!(encoding.as_str(), "auto" | "utf-16le") {
        let (text, _, _) = UTF_16LE.decode(&bytes[2..]);
        return Some(text);
    }
    if bytes.starts_with(&[0xfe, 0xff]) && matches!(encoding.as_str(), "auto" | "utf-16be") {
        let (text, _, _) = UTF_16BE.decode(&bytes[2..]);
        return Some(text);
    }
    let codec = match encoding.as_str() {
        "utf-16le" => UTF_16LE,
        "utf-16be" => UTF_16BE,
        "auto" | "gb18030" => GB18030,
        _ => return None,
    };
    let (text, _, _) = codec.decode(bytes);
    Some(text)
}

fn is_binary(bytes: &[u8], encoding: &str) -> bool {
    if matches!(encoding, "utf-16le" | "utf-16be")
        || bytes.starts_with(&[0xff, 0xfe])
        || bytes.starts_with(&[0xfe, 0xff])
    {
        return false;
    }
    bytes.iter().take(8192).any(|byte| *byte == 0)
}

fn preview(line: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let match_start = line[..start].chars().count();
    let match_end = match_start + line[start..end].chars().count();
    let start = match_start.saturating_sub(80);
    let end = (match_end + 120).min(chars.len());
    let mut text = String::new();
    if start > 0 {
        text.push('…');
    }
    text.extend(&chars[start..end]);
    if end < chars.len() {
        text.push('…');
    }
    text.trim().to_owned()
}

fn emit_progress(bus: &EventBus, task_id: &str, files: u64, matches: u64) {
    emit(
        bus,
        AppEvent::Tool(ToolEventDto::SearchProgress {
            task_id: task_id.to_owned(),
            files,
            matches,
        }),
    );
}

fn emit_hits(bus: &EventBus, task_id: &str, batch: &mut Vec<SearchHitDto>) {
    if batch.is_empty() {
        return;
    }
    emit(
        bus,
        AppEvent::Tool(ToolEventDto::SearchHits {
            task_id: task_id.to_owned(),
            hits: std::mem::take(batch),
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_and_binary_detection() {
        assert_eq!(
            decode_bytes(b"hello", "utf-8").expect("utf8"),
            Cow::Borrowed("hello")
        );
        assert!(is_binary(b"abc\0def", "auto"));
        assert!(!is_binary(b"hello", "auto"));
        assert!(!is_binary(&[0xff, 0xfe, b'h', 0, b'i', 0], "auto"));
    }

    #[test]
    fn compiles_globs_and_rejects_invalid_patterns() {
        let set = build_globset("*.rs", &["node_modules/**"])
            .expect("globs")
            .expect("set");
        assert!(set.is_match("src/main.rs"));
        assert!(set.is_match("node_modules/foo/index.js"));
        assert!(build_globset("[", &[]).is_err());
    }

    #[test]
    fn previews_match_with_context() {
        let line = "prefix-value-suffix";
        assert!(preview(line, 7, 12).contains("value"));
    }

    #[test]
    fn searches_temp_folder() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("a.txt"), "hello world\nHELLO rust\n").expect("write");
        std::fs::write(temp.path().join("skip.bin"), b"hello\0world").expect("write");
        let bus = crate::event::event_bus();
        let cancel = AtomicBool::new(false);
        let input = SearchInput {
            folder: temp.path().to_string_lossy().into_owned(),
            query: "hello".to_owned(),
            mode: "literal".to_owned(),
            case_sensitive: false,
            include: String::new(),
            exclude: String::new(),
            encoding: "auto".to_owned(),
            max_file_size: 1024 * 1024,
            max_results: 100,
            skip_hidden: true,
        };
        let outcome = search_blocking("test", &input, &cancel, &bus).expect("search");
        assert_eq!(outcome.files, 1);
        assert_eq!(outcome.matches, 2);
        assert_eq!(outcome.skipped, 1);
    }

    #[test]
    fn searches_utf16_with_bom() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("utf16.txt"),
            [
                0xff, 0xfe, b'n', 0, b'e', 0, b'e', 0, b'd', 0, b'l', 0, b'e', 0,
            ],
        )
        .expect("write");
        let bus = crate::event::event_bus();
        let cancel = AtomicBool::new(false);
        let input = SearchInput {
            folder: temp.path().to_string_lossy().into_owned(),
            query: "needle".to_owned(),
            mode: "literal".to_owned(),
            case_sensitive: false,
            include: String::new(),
            exclude: String::new(),
            encoding: "auto".to_owned(),
            max_file_size: 1024 * 1024,
            max_results: 100,
            skip_hidden: true,
        };
        let outcome = search_blocking("test", &input, &cancel, &bus).expect("search");
        assert_eq!(outcome.matches, 1);
    }
}
