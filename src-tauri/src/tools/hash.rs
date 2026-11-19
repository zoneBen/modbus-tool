use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use tokio::io::AsyncReadExt;

use crate::event::{AppEvent, EventBus, emit};
use crate::tools::event::ToolEventDto;
use crate::tools::model::{HashDoneDto, HashInput, HashResultDto, HashValueDto, ToolErrorDto};
use crate::tools::tasks::ToolTasks;

const CHUNK_SIZE: usize = 1024 * 1024;
const PROGRESS_STEP: u64 = 8 * 1024 * 1024;

pub async fn run(
    task_id: String,
    input: HashInput,
    cancel: Arc<AtomicBool>,
    tasks: ToolTasks,
    bus: EventBus,
) {
    let result = hash_file(&task_id, &input, &cancel, &bus).await;
    tasks.finish(&task_id);
    let (result, error) = match result {
        Ok(result) => (Some(result), None),
        Err(error) => (None, Some(error)),
    };
    emit(
        &bus,
        AppEvent::Tool(ToolEventDto::HashDone(HashDoneDto {
            task_id,
            result,
            error,
        })),
    );
}

async fn hash_file(
    task_id: &str,
    input: &HashInput,
    cancel: &AtomicBool,
    bus: &EventBus,
) -> Result<HashResultDto, ToolErrorDto> {
    let started = Instant::now();
    let path = PathBuf::from(input.path.trim());
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|error| ToolErrorDto::new("hashReadFailed", error.to_string()))?;
    if !metadata.is_file() {
        return Err(ToolErrorDto::new("notAFile", "Selected path is not a file"));
    }
    let size = metadata.len();
    let algorithms = normalize_algorithms(&input.algorithms)?;
    let mut hashers = algorithms
        .iter()
        .map(|algorithm| Hasher::new(algorithm))
        .collect::<Result<Vec<_>, _>>()?;
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|error| ToolErrorDto::new("hashReadFailed", error.to_string()))?;
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut done = 0u64;
    let mut last_progress = 0u64;
    emit(
        bus,
        AppEvent::Tool(ToolEventDto::HashProgress {
            task_id: task_id.to_owned(),
            done: 0,
            total: size,
        }),
    );
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(ToolErrorDto::new("cancelled", "Hash cancelled"));
        }
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|error| ToolErrorDto::new("hashReadFailed", error.to_string()))?;
        if read == 0 {
            break;
        }
        for hasher in &mut hashers {
            hasher.update(&buffer[..read]);
        }
        done += read as u64;
        if done.saturating_sub(last_progress) >= PROGRESS_STEP || done == size {
            last_progress = done;
            emit(
                bus,
                AppEvent::Tool(ToolEventDto::HashProgress {
                    task_id: task_id.to_owned(),
                    done,
                    total: size,
                }),
            );
        }
    }
    let values = hashers
        .into_iter()
        .map(|hasher| HashValueDto {
            algorithm: hasher.label().to_owned(),
            digest: hasher.finish(),
        })
        .collect();
    Ok(HashResultDto {
        path: path.to_string_lossy().into_owned(),
        size,
        elapsed_ms: round1(started.elapsed().as_secs_f64() * 1000.0),
        values,
    })
}

fn normalize_algorithms(values: &[String]) -> Result<Vec<String>, ToolErrorDto> {
    let mut result = Vec::new();
    for value in values {
        let algorithm = value.trim().to_ascii_lowercase();
        if algorithm.is_empty() || result.contains(&algorithm) {
            continue;
        }
        if !matches!(
            algorithm.as_str(),
            "md5" | "sha1" | "sha256" | "sha512" | "crc32"
        ) {
            return Err(ToolErrorDto::new(
                "unsupportedHash",
                format!("Unsupported hash algorithm: {algorithm}"),
            ));
        }
        result.push(algorithm);
    }
    if result.is_empty() {
        return Err(ToolErrorDto::new(
            "noAlgorithms",
            "Select at least one hash algorithm",
        ));
    }
    Ok(result)
}

enum Hasher {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
    Crc32(crc32fast::Hasher),
}

impl Hasher {
    fn new(algorithm: &str) -> Result<Self, ToolErrorDto> {
        Ok(match algorithm {
            "md5" => Self::Md5(Md5::new()),
            "sha1" => Self::Sha1(Sha1::new()),
            "sha256" => Self::Sha256(Sha256::new()),
            "sha512" => Self::Sha512(Sha512::new()),
            "crc32" => Self::Crc32(crc32fast::Hasher::new()),
            _ => {
                return Err(ToolErrorDto::new(
                    "unsupportedHash",
                    format!("Unsupported hash algorithm: {algorithm}"),
                ));
            }
        })
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Md5(hasher) => hasher.update(bytes),
            Self::Sha1(hasher) => hasher.update(bytes),
            Self::Sha256(hasher) => hasher.update(bytes),
            Self::Sha512(hasher) => hasher.update(bytes),
            Self::Crc32(hasher) => hasher.update(bytes),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Md5(_) => "MD5",
            Self::Sha1(_) => "SHA-1",
            Self::Sha256(_) => "SHA-256",
            Self::Sha512(_) => "SHA-512",
            Self::Crc32(_) => "CRC32",
        }
    }

    fn finish(self) -> String {
        match self {
            Self::Md5(hasher) => hex::encode(hasher.finalize()),
            Self::Sha1(hasher) => hex::encode(hasher.finalize()),
            Self::Sha256(hasher) => hex::encode(hasher.finalize()),
            Self::Sha512(hasher) => hex::encode(hasher.finalize()),
            Self::Crc32(hasher) => format!("{:08X}", hasher.finalize()),
        }
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_rejects_algorithms() {
        assert_eq!(
            normalize_algorithms(&["SHA256".to_owned(), "md5".to_owned(), "sha256".to_owned()])
                .expect("algorithms"),
            vec!["sha256", "md5"]
        );
        assert!(normalize_algorithms(&[]).is_err());
        assert!(normalize_algorithms(&["nope".to_owned()]).is_err());
    }

    #[test]
    fn hashes_known_vector() {
        let mut md5 = Hasher::new("md5").expect("md5");
        md5.update(b"abc");
        assert_eq!(md5.finish(), "900150983cd24fb0d6963f7d28e17f72");

        let mut sha256 = Hasher::new("sha256").expect("sha256");
        sha256.update(b"abc");
        assert_eq!(
            sha256.finish(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
