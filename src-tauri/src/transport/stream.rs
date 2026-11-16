//! Framing-aware reading from an async byte stream.

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::time::{Instant, timeout_at};

use modbus_core::Framing;
use modbus_core::frame::{self, MAX_ASCII_FRAME_LEN, MAX_RTU_ADU_LEN};

use super::TransportError;

pub async fn read_frame<S: AsyncRead + Unpin>(
    stream: &mut S,
    framing: Framing,
    deadline: Instant,
) -> Result<Vec<u8>, TransportError> {
    match framing {
        Framing::Mbap => read_mbap(stream, deadline).await,
        Framing::Rtu => read_rtu(stream, deadline).await,
        Framing::Ascii => read_ascii(stream, deadline).await,
    }
}

async fn read_mbap<S: AsyncRead + Unpin>(
    stream: &mut S,
    deadline: Instant,
) -> Result<Vec<u8>, TransportError> {
    let mut frame = Vec::with_capacity(16);
    read_exact_deadline(stream, &mut frame, 6, deadline).await?;
    let length = u16::from_be_bytes([frame[4], frame[5]]);
    if !(2..=254).contains(&length) {
        return Err(TransportError::InvalidFrame(format!(
            "非法 MBAP 长度 {length}"
        )));
    }
    let total = 6 + length as usize;
    read_exact_deadline(stream, &mut frame, total, deadline).await?;
    Ok(frame)
}

async fn read_rtu<S: AsyncRead + Unpin>(
    stream: &mut S,
    deadline: Instant,
) -> Result<Vec<u8>, TransportError> {
    let mut frame = Vec::with_capacity(16);
    loop {
        if let Some(expected) = frame::expected_frame_len(Framing::Rtu, &frame) {
            if frame.len() >= expected {
                frame.truncate(expected);
                return Ok(frame);
            }
        } else if frame.len() >= 8 {
            return Err(TransportError::InvalidFrame(
                "无法从功能码推导 RTU 响应长度".to_owned(),
            ));
        }
        let needed = frame.len() + 1;
        read_exact_deadline(stream, &mut frame, needed, deadline).await?;
        if frame.len() > MAX_RTU_ADU_LEN {
            return Err(TransportError::InvalidFrame("RTU 帧超长".to_owned()));
        }
    }
}

async fn read_ascii<S: AsyncRead + Unpin>(
    stream: &mut S,
    deadline: Instant,
) -> Result<Vec<u8>, TransportError> {
    let mut frame = Vec::with_capacity(16);
    loop {
        let needed = frame.len() + 1;
        read_exact_deadline(stream, &mut frame, needed, deadline).await?;
        if frame.last() == Some(&b'\n') {
            return Ok(frame);
        }
        if frame.len() >= MAX_ASCII_FRAME_LEN {
            return Err(TransportError::InvalidFrame("ASCII 帧超长".to_owned()));
        }
    }
}

async fn read_exact_deadline<S: AsyncRead + Unpin>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    needed: usize,
    deadline: Instant,
) -> Result<(), TransportError> {
    while buffer.len() < needed {
        let mut byte = [0u8; 1];
        match timeout_at(deadline, stream.read(&mut byte)).await {
            Err(_) => {
                return Err(TransportError::Timeout {
                    partial: buffer.clone(),
                });
            }
            Ok(Ok(0)) => return Err(TransportError::Closed),
            Ok(Ok(read)) => buffer.extend_from_slice(&byte[..read]),
            Ok(Err(error)) => return Err(TransportError::Io(error.to_string())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn reads_mbap_frame() {
        let (mut client, mut server) = duplex(64);
        let frame = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00, 0x00, 0x00, 0x10,
        ];
        let expected = frame.clone();
        tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_all(&mut server, &frame)
                .await
                .expect("write");
        });
        let received = read_frame(
            &mut client,
            Framing::Mbap,
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .await
        .expect("read");
        assert_eq!(received, expected);
    }

    #[tokio::test]
    async fn reads_rtu_frame_by_expected_length() {
        let (mut client, mut server) = duplex(64);
        let frame = vec![0x01, 0x03, 0x04, 0x00, 0x2A, 0x00, 0x2B, 0x00, 0x00];
        let expected = frame.clone();
        tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_all(&mut server, &frame)
                .await
                .expect("write");
        });
        let received = read_frame(
            &mut client,
            Framing::Rtu,
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .await
        .expect("read");
        assert_eq!(received[..9], expected[..9]);
    }

    #[tokio::test]
    async fn reads_ascii_until_newline() {
        let (mut client, mut server) = duplex(64);
        let frame = b":01030000000AF2\r\n".to_vec();
        let expected = frame.clone();
        tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_all(&mut server, &frame)
                .await
                .expect("write");
        });
        let received = read_frame(
            &mut client,
            Framing::Ascii,
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .await
        .expect("read");
        assert_eq!(received, expected);
    }
}
