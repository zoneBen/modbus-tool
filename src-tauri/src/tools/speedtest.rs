use std::time::{Duration, Instant};

use super::model::{SpeedTestInput, SpeedTestResultDto, ToolErrorDto, ToolResult};

const SERVER_LABEL: &str = "speed.cloudflare.com";
const DOWN_URL: &str = "https://speed.cloudflare.com/__down?bytes=";
const UP_URL: &str = "https://speed.cloudflare.com/__up";
const MAX_BYTES: u64 = 200 * 1024 * 1024;

pub async fn run(input: SpeedTestInput) -> ToolResult<SpeedTestResultDto> {
    let started = Instant::now();
    let download_bytes = input.download_bytes.clamp(1_000_000, MAX_BYTES);
    let upload_bytes = input.upload_bytes.clamp(512_000, 100 * 1024 * 1024);
    let samples = input.ping_samples.clamp(3, 20);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| ToolErrorDto::new("clientInit", error.to_string()))?;

    let (latency_ms, jitter_ms) = measure_latency(&client, samples).await?;
    let download_mbps = measure_download(&client, download_bytes).await?;
    let upload_mbps = measure_upload(&client, upload_bytes).await?;

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(SpeedTestResultDto {
        server: SERVER_LABEL.to_owned(),
        download_mbps: round(download_mbps, 2),
        upload_mbps: round(upload_mbps, 2),
        latency_ms: round(latency_ms, 1),
        jitter_ms: round(jitter_ms, 1),
        download_bytes,
        upload_bytes,
        elapsed_ms: round(elapsed_ms, 0),
    })
}

async fn measure_latency(client: &reqwest::Client, samples: u8) -> ToolResult<(f64, f64)> {
    let mut values = Vec::new();
    for _ in 0..samples {
        let started = Instant::now();
        let response = client
            .get(format!("{DOWN_URL}0"))
            .send()
            .await
            .map_err(|error| ToolErrorDto::new("speedLatency", error.to_string()))?;
        let _ = response.bytes().await;
        values.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    if values.is_empty() {
        return Err(ToolErrorDto::new("speedLatency", "No latency samples"));
    }
    let average = values.iter().sum::<f64>() / values.len() as f64;
    let jitter = if values.len() > 1 {
        values
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .sum::<f64>()
            / (values.len() - 1) as f64
    } else {
        0.0
    };
    Ok((average, jitter))
}

async fn measure_download(client: &reqwest::Client, bytes: u64) -> ToolResult<f64> {
    let started = Instant::now();
    let mut response = client
        .get(format!("{DOWN_URL}{bytes}"))
        .send()
        .await
        .map_err(|error| ToolErrorDto::new("speedDownload", error.to_string()))?;
    if !response.status().is_success() {
        return Err(ToolErrorDto::new(
            "speedDownload",
            format!("Download endpoint returned HTTP {}", response.status()),
        ));
    }
    let mut received = 0u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ToolErrorDto::new("speedDownload", error.to_string()))?
    {
        received += chunk.len() as u64;
    }
    let seconds = started.elapsed().as_secs_f64().max(0.001);
    if received == 0 {
        return Err(ToolErrorDto::new("speedDownload", "No data received"));
    }
    Ok((received as f64 * 8.0) / seconds / 1_000_000.0)
}

async fn measure_upload(client: &reqwest::Client, bytes: u64) -> ToolResult<f64> {
    let payload = vec![0u8; bytes as usize];
    let started = Instant::now();
    let response = client
        .post(UP_URL)
        .header("Content-Type", "application/octet-stream")
        .body(payload)
        .send()
        .await
        .map_err(|error| ToolErrorDto::new("speedUpload", error.to_string()))?;
    if !response.status().is_success() {
        return Err(ToolErrorDto::new(
            "speedUpload",
            format!("Upload endpoint returned HTTP {}", response.status()),
        ));
    }
    let seconds = started.elapsed().as_secs_f64().max(0.001);
    Ok((bytes as f64 * 8.0) / seconds / 1_000_000.0)
}

fn round(value: f64, digits: i32) -> f64 {
    let factor = 10f64.powi(digits);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_are_applied_to_request_sizes() {
        let input = SpeedTestInput {
            download_bytes: 10,
            upload_bytes: 1,
            ping_samples: 1,
        };
        let download = input.download_bytes.clamp(1_000_000, MAX_BYTES);
        let upload = input.upload_bytes.clamp(512_000, 100 * 1024 * 1024);
        assert_eq!(download, 1_000_000);
        assert_eq!(upload, 512_000);
    }

    #[test]
    fn rounds_to_requested_digits() {
        assert_eq!(round(12.3456, 2), 12.35);
        assert_eq!(round(12.3456, 1), 12.3);
    }
}
