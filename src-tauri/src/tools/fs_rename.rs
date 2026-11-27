use std::collections::HashSet;
use std::path::Path;
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use super::model::{
    FsEntryDto, FsListInput, FsListResultDto, RenameInput, RenameItemDto, RenameResultDto,
    ToolErrorDto, ToolResult,
};

pub fn list(input: FsListInput) -> ToolResult<FsListResultDto> {
    let path = input.path.trim();
    if path.is_empty() {
        return Err(ToolErrorDto::new("invalidPath", "Directory is required"));
    }
    let root = Path::new(path);
    if !root.is_dir() {
        return Err(ToolErrorDto::new(
            "notADirectory",
            "The selected path is not a directory",
        ));
    }
    let limit = input.limit.clamp(1, 20_000);
    let depth = if input.recursive { usize::MAX } else { 1 };
    let mut entries = Vec::new();
    let mut truncated = false;

    for entry in WalkDir::new(root).max_depth(depth).follow_links(false) {
        let Ok(entry) = entry else { continue };
        if entry.depth() == 0 {
            continue;
        }
        if entries.len() >= limit {
            truncated = true;
            break;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
        entries.push(FsEntryDto {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified_ms,
        });
    }

    Ok(FsListResultDto {
        path: root.to_string_lossy().into_owned(),
        entries,
        truncated,
    })
}

fn same_file(from: &Path, to: &Path) -> bool {
    match (from.canonicalize(), to.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub fn rename(input: RenameInput) -> ToolResult<RenameResultDto> {
    if input.pairs.is_empty() {
        return Err(ToolErrorDto::new("noOperations", "No rename operations"));
    }
    let mut targets = HashSet::new();
    let mut items = Vec::with_capacity(input.pairs.len());
    let mut renamed = 0usize;
    let mut failed = 0usize;

    for pair in &input.pairs {
        let from = Path::new(&pair.from);
        let to = Path::new(&pair.to);
        let mut status = "ok";
        let mut detail = None;

        if pair.from.trim().is_empty() || pair.to.trim().is_empty() {
            status = "invalid";
            detail = Some("Empty path".to_owned());
        } else if !from.exists() {
            status = "missing";
            detail = Some("Source does not exist".to_owned());
        } else if from == to {
            status = "skipped";
            detail = Some("Name unchanged".to_owned());
        } else if !targets.insert(pair.to.clone()) {
            status = "conflict";
            detail = Some("Duplicate target in this batch".to_owned());
        } else if to.exists() && !same_file(from, to) {
            status = "conflict";
            detail = Some("Target already exists".to_owned());
        } else if let Some(parent) = to.parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            status = "invalid";
            detail = Some("Target directory does not exist".to_owned());
        }

        if !input.dry_run && status == "ok" {
            let outcome = if same_file(from, to) {
                // Case-only rename on a case-insensitive filesystem: move aside
                // first so Windows accepts the target name.
                let temp =
                    from.with_file_name(format!(".mt-rename-{}", uuid::Uuid::new_v4().simple()));
                std::fs::rename(from, &temp).and_then(|()| std::fs::rename(&temp, to))
            } else {
                std::fs::rename(from, to)
            };
            match outcome {
                Ok(()) => renamed += 1,
                Err(error) => {
                    failed += 1;
                    status = "error";
                    detail = Some(error.to_string());
                }
            }
        }

        items.push(RenameItemDto {
            from: pair.from.clone(),
            to: pair.to.clone(),
            status: status.to_owned(),
            detail,
        });
    }

    Ok(RenameResultDto {
        applied: !input.dry_run,
        renamed,
        failed,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_and_renames_files_in_a_temp_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join("a.log");
        fs::write(&first, "one").expect("write");
        fs::write(dir.path().join("b.log"), "two").expect("write");

        let listed = list(FsListInput {
            path: dir.path().to_string_lossy().into_owned(),
            recursive: false,
            limit: 100,
        })
        .expect("list");
        assert_eq!(listed.entries.len(), 2);

        let target = dir.path().join("a-renamed.log");
        let result = rename(RenameInput {
            pairs: vec![super::super::model::RenamePairInput {
                from: first.to_string_lossy().into_owned(),
                to: target.to_string_lossy().into_owned(),
            }],
            dry_run: false,
        })
        .expect("rename");
        assert_eq!(result.renamed, 1);
        assert!(target.exists());
    }

    #[test]
    fn reports_conflicts_without_touching_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("a.log");
        let occupied = dir.path().join("b.log");
        fs::write(&source, "one").expect("write");
        fs::write(&occupied, "two").expect("write");

        let result = rename(RenameInput {
            pairs: vec![super::super::model::RenamePairInput {
                from: source.to_string_lossy().into_owned(),
                to: occupied.to_string_lossy().into_owned(),
            }],
            dry_run: false,
        })
        .expect("rename");
        assert_eq!(result.items[0].status, "conflict");
        assert_eq!(result.failed, 0);
        assert!(source.exists());
    }
}
