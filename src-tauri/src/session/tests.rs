use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use modbus_core::frame;
use modbus_core::simulator::{Simulator, SlaveUnit};
use modbus_core::{DataType, Framing, OrderPreset, PduRequest, PduResponse};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::event::{AppEvent, event_bus};
use crate::model::{
    CustomWriteInput, DefinitionInput, NewSessionInput, RawSendInput, RawSendMode, SessionRole,
    SlaveValueInput, TransportKind,
};
use crate::session::SessionManager;

async fn mock_tcp_slave(listener: TcpListener, simulator: Arc<Mutex<Simulator>>) {
    loop {
        let Ok((mut socket, _)) = listener.accept().await else {
            break;
        };
        let simulator = simulator.clone();
        tokio::spawn(async move {
            loop {
                let mut header = [0u8; 6];
                if socket.read_exact(&mut header).await.is_err() {
                    break;
                }
                let length = usize::from(u16::from_be_bytes([header[4], header[5]]));
                let mut rest = vec![0u8; length];
                if socket.read_exact(&mut rest).await.is_err() {
                    break;
                }
                let mut request = header.to_vec();
                request.extend_from_slice(&rest);
                let Ok(adu) = frame::decode(Framing::Mbap, &request) else {
                    break;
                };
                let response = {
                    let mut simulator = simulator.lock().await;
                    simulator.handle_request(adu.unit_id, &adu.pdu)
                };
                if let Some(pdu) = response {
                    let Ok(response) =
                        frame::encode(Framing::Mbap, adu.unit_id, adu.transaction_id, &pdu)
                    else {
                        break;
                    };
                    if socket.write_all(&response).await.is_err() {
                        break;
                    }
                }
            }
        });
    }
}

fn definition_input(name: &str) -> DefinitionInput {
    DefinitionInput {
        name: name.to_owned(),
        function_code: 0x03,
        slave_id: 1,
        start_address: 1,
        quantity: 1,
        data_type: DataType::U16,
        byte_order: OrderPreset::Abcd,
        byte_pos: None,
        interval_ms: 1000,
        enabled: true,
        aliases: BTreeMap::new(),
    }
}

fn input(port: u16) -> NewSessionInput {
    NewSessionInput {
        name: "TCP integration".to_owned(),
        role: SessionRole::Master,
        transport: TransportKind::Tcp,
        framing: Framing::Mbap,
        endpoint: format!("127.0.0.1:{port}"),
        host: Some("127.0.0.1".to_owned()),
        port: Some(port),
        serial_port: None,
        baud_rate: None,
        data_bits: None,
        parity: None,
        stop_bits: None,
        timeout_ms: Some(500),
        retries: Some(0),
        inter_frame_delay_ms: Some(0),
        auto_reconnect: Some(false),
        slave_id: None,
    }
}

#[test]
fn encodes_manual_send_payloads() {
    let ascii = RawSendInput {
        data: "AT+INFO\r\n".to_owned(),
        mode: RawSendMode::Ascii,
    };
    assert_eq!(ascii.encode().expect("ascii"), b"AT+INFO\r\n");

    let hex = RawSendInput {
        data: "0x01 03, 00-0A".to_owned(),
        mode: RawSendMode::Hex,
    };
    assert_eq!(hex.encode().expect("hex"), vec![0x01, 0x03, 0x00, 0x0A]);

    for (data, mode) in [
        ("", RawSendMode::Ascii),
        ("0", RawSendMode::Hex),
        ("0G", RawSendMode::Hex),
        ("  ", RawSendMode::Hex),
    ] {
        let input = RawSendInput {
            data: data.to_owned(),
            mode,
        };
        assert!(input.encode().is_err(), "payload should be rejected");
    }
}

async fn echo_tcp_server(listener: TcpListener) {
    loop {
        let Ok((mut socket, _)) = listener.accept().await else {
            break;
        };
        tokio::spawn(async move {
            let mut buffer = [0u8; 512];
            loop {
                match socket.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if socket.write_all(&buffer[..read]).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }
}

#[tokio::test]
async fn sends_and_logs_manual_raw_data_over_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server = tokio::spawn(echo_tcp_server(listener));

    let bus = event_bus();
    let mut events = bus.subscribe();
    let manager = SessionManager::new(bus);
    let session = manager.create_session(input(port)).await.expect("create");

    let result = manager
        .send_raw(
            &session.id,
            RawSendInput {
                data: "PING".to_owned(),
                mode: RawSendMode::Ascii,
            },
        )
        .await
        .expect("raw send");
    assert_eq!(result.bytes_sent, 4);
    assert_eq!(result.bytes_received, 4);
    assert!(result.latency_ms.is_finite());

    let mut frames = Vec::new();
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::SessionFrames {
                frames: mut batch, ..
            })) => {
                frames.append(&mut batch);
                if frames
                    .iter()
                    .any(|frame| frame.summary.contains("手动发送"))
                    && frames
                        .iter()
                        .any(|frame| frame.summary.contains("手动接收"))
                {
                    break;
                }
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        frames.iter().any(|frame| frame.summary.contains("手动发送")
            && frame.direction == "tx"
            && frame.raw.as_slice() == b"PING"),
        "manual TX frame should be logged"
    );
    assert!(
        frames.iter().any(|frame| frame.summary.contains("手动接收")
            && frame.direction == "rx"
            && frame.raw.as_slice() == b"PING"),
        "manual RX frame should be logged"
    );

    manager.shutdown_all().await;
    server.abort();
}

#[tokio::test]
async fn reads_and_writes_over_real_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let mut read_only = SlaveUnit::new(2).with_holding_registers([(0, 0)]);
    read_only.write_enabled = false;
    let simulator = Arc::new(Mutex::new(Simulator::new([
        SlaveUnit::new(1).with_holding_registers([(0, 0x1234), (1, 0xABCD)]),
        read_only,
    ])));
    let server = tokio::spawn(mock_tcp_slave(listener, simulator.clone()));

    let bus = event_bus();
    let mut events = bus.subscribe();
    let manager = SessionManager::new(bus);
    let session = manager.create_session(input(port)).await.expect("create");
    assert!(
        session.definitions.is_empty(),
        "new sessions must not seed read definitions"
    );
    let mut read_input = definition_input("读取定义");
    read_input.start_address = 0;
    read_input.quantity = 2;
    let definition = manager
        .create_definition(&session.id, read_input)
        .await
        .expect("create read definition");
    let definition_id = definition.id.clone();

    manager
        .read_once(&session.id, &definition_id)
        .await
        .expect("read");

    let mut data = None;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::SessionData(snapshot)))
                if snapshot.definition_id == definition_id && !snapshot.values.is_empty() =>
            {
                data = Some(snapshot);
                break;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => break,
        }
    }
    let data = data.expect("session data event");
    assert_eq!(data.values[0].numeric, Some(4660.0));
    assert_eq!(data.values[1].numeric, Some(43981.0));

    manager
        .write_register(&session.id, &definition_id, 1, "1234")
        .await
        .expect("write");
    let written = simulator.lock().await.unit(1).expect("unit").holding(1);
    assert_eq!(written, 1234);

    manager
        .write_custom(
            &session.id,
            Some(&definition_id),
            CustomWriteInput {
                slave_id: 1,
                function_code: 0x10,
                address: 0,
                values: vec!["111".to_owned(), "0x00DE".to_owned()],
            },
        )
        .await
        .expect("custom write");
    assert_eq!(
        simulator.lock().await.unit(1).expect("unit").holding(0),
        111
    );
    assert_eq!(
        simulator.lock().await.unit(1).expect("unit").holding(1),
        222
    );

    let error = manager
        .write_custom(
            &session.id,
            Some(&definition_id),
            CustomWriteInput {
                slave_id: 2,
                function_code: 0x06,
                address: 0,
                values: vec!["1".to_owned()],
            },
        )
        .await
        .expect_err("out-of-range write must fail");
    assert!(
        error.contains("Modbus 异常 0x02") && error.contains("地址非法"),
        "error should explain the Modbus exception: {error}"
    );

    let mut bad_input = definition_input("无响应定义");
    bad_input.slave_id = 9;
    let bad_definition = manager
        .create_definition(&session.id, bad_input)
        .await
        .expect("create bad definition");
    manager
        .set_definition_enabled(&session.id, &bad_definition.id, false)
        .await
        .expect("disable bad definition");
    let _ = manager
        .read_once(&session.id, &bad_definition.id)
        .await
        .expect_err("missing slave must time out");
    let mut error_snapshot = None;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::SessionData(snapshot)))
                if snapshot.definition_id == bad_definition.id && snapshot.error.is_some() =>
            {
                error_snapshot = Some(snapshot);
                break;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => break,
        }
    }
    let error_snapshot = error_snapshot.expect("error snapshot event");
    assert_eq!(error_snapshot.values.len(), 1);
    assert_eq!(error_snapshot.values[0].formatted, "—");
    assert_eq!(error_snapshot.values[0].numeric, None);
    manager
        .delete_definition(&session.id, &bad_definition.id)
        .await
        .expect("delete bad definition");

    let created = manager
        .create_definition(&session.id, definition_input("额外定义"))
        .await
        .expect("create definition");
    assert_eq!(created.quantity, 1);
    let updated = manager
        .update_definition(&session.id, &created.id, definition_input("重命名定义"))
        .await
        .expect("update definition");
    assert_eq!(updated.name, "重命名定义");
    manager
        .delete_definition(&session.id, &created.id)
        .await
        .expect("delete definition");

    manager.shutdown_all().await;
    server.abort();
}

#[tokio::test]
async fn tcp_slave_serves_modbus_requests() {
    let probe = TcpListener::bind("127.0.0.1:0").await.expect("probe bind");
    let port = probe.local_addr().expect("probe addr").port();
    drop(probe);

    let mut slave_input = input(port);
    slave_input.name = "TCP slave".to_owned();
    slave_input.role = SessionRole::Slave;
    slave_input.slave_id = Some(1);
    let manager = SessionManager::new(event_bus());
    let session = manager
        .create_session(slave_input)
        .await
        .expect("start slave");
    assert_eq!(session.role, SessionRole::Slave);
    assert_eq!(session.definitions.len(), 1);

    let connect = TcpStream::connect(("127.0.0.1", port));
    let mut socket = tokio::time::timeout(Duration::from_secs(2), connect)
        .await
        .expect("connect timeout")
        .expect("connect slave");

    let read = PduRequest::ReadHoldingRegisters {
        address: 0,
        quantity: 2,
    };
    let pdu = read.encode().expect("request pdu");
    let request_frame = frame::encode(Framing::Mbap, 1, Some(7), &pdu).expect("request frame");
    socket
        .write_all(&request_frame)
        .await
        .expect("write request");
    let mut header = [0u8; 6];
    tokio::time::timeout(Duration::from_secs(2), socket.read_exact(&mut header))
        .await
        .expect("header timeout")
        .expect("read header");
    let length = usize::from(u16::from_be_bytes([header[4], header[5]]));
    let mut response = header.to_vec();
    response.resize(6 + length, 0);
    let body = socket.read_exact(&mut response[6..]);
    tokio::time::timeout(Duration::from_secs(2), body)
        .await
        .expect("body timeout")
        .expect("read body");
    let adu = frame::decode(Framing::Mbap, &response).expect("decode response");
    let decoded = PduResponse::decode_for_request(&read, &adu.pdu).expect("response pdu");
    assert_eq!(
        decoded,
        PduResponse::ReadRegisters {
            function_code: 3,
            values: vec![1000, 1001]
        }
    );

    let written = manager
        .slave_write_value(
            &session.id,
            SlaveValueInput {
                address: 0,
                data_type: DataType::U16,
                byte_order: OrderPreset::Abcd,
                value: "4660".to_owned(),
            },
        )
        .await
        .expect("write simulated value");
    assert_eq!(written, 1);

    let mut header = [0u8; 6];
    let request_frame = frame::encode(Framing::Mbap, 1, Some(8), &pdu).expect("request frame");
    socket
        .write_all(&request_frame)
        .await
        .expect("write second request");
    tokio::time::timeout(Duration::from_secs(2), socket.read_exact(&mut header))
        .await
        .expect("header timeout")
        .expect("read second header");
    let length = usize::from(u16::from_be_bytes([header[4], header[5]]));
    let mut response = header.to_vec();
    response.resize(6 + length, 0);
    let body = socket.read_exact(&mut response[6..]);
    tokio::time::timeout(Duration::from_secs(2), body)
        .await
        .expect("body timeout")
        .expect("read second body");
    let adu = frame::decode(Framing::Mbap, &response).expect("decode second response");
    let decoded = PduResponse::decode_for_request(&read, &adu.pdu).expect("second response pdu");
    assert_eq!(
        decoded,
        PduResponse::ReadRegisters {
            function_code: 3,
            values: vec![4660, 1001]
        }
    );

    manager.shutdown_all().await;
}
