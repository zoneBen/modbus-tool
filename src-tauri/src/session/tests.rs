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
    AlertOperator, AlertRecoveryDto, AlertRuleDto, AlertSeverity, CustomWriteInput,
    DefinitionInput, NewSessionInput, RawSendInput, RawSendMode, RegisterPointDto, SessionRole,
    SessionStatus, SimulatorMode, SimulatorPointDto, SlaveValueInput, TransportKind,
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
        points: Vec::new(),
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
        auto_connect: true,
    }
}

#[test]
fn encodes_manual_send_payloads() {
    let ascii = RawSendInput {
        data: "AT+INFO\r\n".to_owned(),
        mode: RawSendMode::Ascii,
        append_cr: false,
        append_lf: false,
    };
    assert_eq!(ascii.encode().expect("ascii"), b"AT+INFO\r\n");

    let hex = RawSendInput {
        data: "0x01 03, 00-0A".to_owned(),
        mode: RawSendMode::Hex,
        append_cr: false,
        append_lf: false,
    };
    assert_eq!(hex.encode().expect("hex"), vec![0x01, 0x03, 0x00, 0x0A]);

    let crlf = RawSendInput {
        data: "ls".to_owned(),
        mode: RawSendMode::Ascii,
        append_cr: true,
        append_lf: true,
    };
    assert_eq!(crlf.encode().expect("crlf"), b"ls\r\n");

    let bare_lf = RawSendInput {
        data: String::new(),
        mode: RawSendMode::Ascii,
        append_cr: false,
        append_lf: true,
    };
    assert_eq!(bare_lf.encode().expect("bare lf"), vec![0x0A]);

    for (data, mode) in [
        ("", RawSendMode::Ascii),
        ("0", RawSendMode::Hex),
        ("0G", RawSendMode::Hex),
        ("  ", RawSendMode::Hex),
    ] {
        let input = RawSendInput {
            data: data.to_owned(),
            mode,
            append_cr: false,
            append_lf: false,
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
async fn debug_mode_reads_continuously_and_sends_manually() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server = tokio::spawn(echo_tcp_server(listener));

    let bus = event_bus();
    let mut events = bus.subscribe();
    let manager = SessionManager::new(bus);
    let session = manager.create_session(input(port)).await.expect("create");

    manager
        .set_polling(&session.id, true)
        .await
        .expect("enable polling");
    let error = manager
        .debug_start(&session.id)
        .await
        .expect_err("polling must block debug mode");
    assert!(error.contains("停止轮询"), "unexpected error: {error}");
    manager
        .set_polling(&session.id, false)
        .await
        .expect("disable polling");

    manager.debug_start(&session.id).await.expect("start debug");
    let error = manager
        .debug_start(&session.id)
        .await
        .expect_err("debug cannot start twice");
    assert!(error.contains("调试模式"), "unexpected error: {error}");

    let sent = manager
        .debug_send(
            &session.id,
            RawSendInput {
                data: "PING".to_owned(),
                mode: RawSendMode::Ascii,
                append_cr: false,
                append_lf: false,
            },
        )
        .await
        .expect("debug send");
    assert_eq!(sent, 4);

    let mut tx = false;
    let mut rx_received = Vec::new();
    let mut active_state = false;
    for _ in 0..40 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::DebugData(data))) => {
                if data.direction == "tx" && data.data.as_slice() == b"PING" {
                    tx = true;
                } else if data.direction == "rx" {
                    rx_received.extend_from_slice(&data.data);
                }
            }
            Ok(Ok(AppEvent::DebugState(state))) => {
                active_state |= state.active;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => break,
        }
        if tx && rx_received.windows(4).any(|window| window == &b"PING"[..]) && active_state {
            break;
        }
    }
    assert!(tx, "manual debug TX should be emitted");
    assert!(
        rx_received.windows(4).any(|window| window == &b"PING"[..]),
        "continuous debug read should emit the echoed bytes"
    );
    assert!(active_state, "debug active state should be emitted");

    manager.debug_stop(&session.id).await.expect("stop debug");
    let error = manager
        .debug_send(
            &session.id,
            RawSendInput {
                data: "PING".to_owned(),
                mode: RawSendMode::Ascii,
                append_cr: false,
                append_lf: false,
            },
        )
        .await
        .expect_err("debug send must stop after debug mode ends");
    assert!(error.contains("未开启"), "unexpected error: {error}");

    let mut stopped_state = false;
    for _ in 0..10 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::DebugState(state))) if !state.active => {
                stopped_state = true;
                break;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(stopped_state, "debug inactive state should be emitted");

    manager.shutdown_all().await;
    server.abort();
}

#[tokio::test]
async fn applies_point_scaling_and_emits_alerts() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let simulator = Arc::new(Mutex::new(Simulator::new([
        SlaveUnit::new(1).with_holding_registers([(0, 100), (1, 0)])
    ])));
    let server = tokio::spawn(mock_tcp_slave(listener, simulator));

    let bus = event_bus();
    let mut events = bus.subscribe();
    let manager = SessionManager::new(bus);
    let session = manager.create_session(input(port)).await.expect("create");
    let mut definition = definition_input("缩放告警");
    definition.start_address = 0;
    definition.quantity = 2;
    definition.points = vec![RegisterPointDto {
        address: 0,
        data_type: DataType::U16,
        byte_order: OrderPreset::Abcd,
        byte_pos: None,
        meaning: "输入电压".to_owned(),
        unit: "V".to_owned(),
        scale: 10.0,
        offset: -5.0,
        alerts: vec![AlertRuleDto {
            id: "alert-1".to_owned(),
            description: "电压过高".to_owned(),
            severity: AlertSeverity::Critical,
            operator: AlertOperator::Gt,
            value: 500.0,
            delay_ms: 0,
            enabled: true,
            recovery: None,
        }],
        simulator: None,
    }];
    let definition = manager
        .create_definition(&session.id, definition)
        .await
        .expect("create definition");
    manager
        .read_once(&session.id, &definition.id)
        .await
        .expect("read");

    let mut scaled_value = None;
    let mut alert_event = None;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::SessionData(snapshot))) if snapshot.definition_id == definition.id => {
                scaled_value = snapshot.values.first().map(|value| value.numeric);
            }
            Ok(Ok(AppEvent::Alert(event))) if event.rule_id == "alert-1" => {
                alert_event = Some(event);
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => break,
        }
        if scaled_value.is_some() && alert_event.is_some() {
            break;
        }
    }
    assert_eq!(scaled_value.flatten(), Some(995.0));
    let alert_event = alert_event.expect("alert event");
    assert!(alert_event.triggered);
    assert_eq!(alert_event.meaning, "输入电压");
    assert_eq!(alert_event.description, "电压过高");
    assert_eq!(alert_event.severity, AlertSeverity::Critical);
    assert_eq!(alert_event.unit, "V");
    assert!((alert_event.value - 995.0).abs() < 1e-6);

    manager.shutdown_all().await;
    server.abort();
}

#[tokio::test]
async fn staged_alert_requires_recovery_condition_before_clearing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let simulator = Arc::new(Mutex::new(Simulator::new([
        SlaveUnit::new(1).with_holding_registers([(0, 100), (1, 0)])
    ])));
    let server = tokio::spawn(mock_tcp_slave(listener, simulator.clone()));

    let bus = event_bus();
    let mut events = bus.subscribe();
    let manager = SessionManager::new(bus);
    let session = manager.create_session(input(port)).await.expect("create");
    manager
        .set_polling(&session.id, false)
        .await
        .expect("stop polling");

    let mut definition = definition_input("恢复规则");
    definition.start_address = 0;
    definition.quantity = 1;
    definition.points = vec![RegisterPointDto {
        address: 0,
        data_type: DataType::U16,
        byte_order: OrderPreset::Abcd,
        byte_pos: None,
        meaning: "温度".to_owned(),
        unit: "C".to_owned(),
        scale: 10.0,
        offset: -5.0,
        alerts: vec![AlertRuleDto {
            id: "alert-recovery".to_owned(),
            description: "电压恢复测试".to_owned(),
            severity: AlertSeverity::Warning,
            operator: AlertOperator::Gt,
            value: 500.0,
            delay_ms: 0,
            enabled: true,
            recovery: Some(AlertRecoveryDto {
                operator: AlertOperator::Lt,
                value: 300.0,
                delay_ms: 200,
            }),
        }],
        simulator: None,
    }];
    let definition = manager
        .create_definition(&session.id, definition)
        .await
        .expect("create definition");

    // 995 > 500: trigger.
    manager
        .read_once(&session.id, &definition.id)
        .await
        .expect("trigger read");
    let mut triggered = false;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::Alert(event))) if event.rule_id == "alert-recovery" => {
                if event.triggered {
                    triggered = true;
                }
            }
            Ok(Ok(AppEvent::SessionData(snapshot))) if snapshot.definition_id == definition.id => {
                break;
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(triggered, "alert should trigger");

    // 495 < 500 but not below the recovery threshold: stay active.
    simulator
        .lock()
        .await
        .unit_mut(1)
        .expect("unit")
        .set_holding(0, 50);
    manager
        .read_once(&session.id, &definition.id)
        .await
        .expect("intermediate read");
    let mut cleared_early = false;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::Alert(event)))
                if event.rule_id == "alert-recovery" && !event.triggered =>
            {
                cleared_early = true;
            }
            Ok(Ok(AppEvent::SessionData(snapshot))) if snapshot.definition_id == definition.id => {
                break;
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        !cleared_early,
        "alert must not clear before recovery threshold"
    );

    // 195 < 500 and < 300: recovery condition starts its 200 ms hold.
    simulator
        .lock()
        .await
        .unit_mut(1)
        .expect("unit")
        .set_holding(0, 20);
    manager
        .read_once(&session.id, &definition.id)
        .await
        .expect("recovery hold read");
    let mut cleared_early = false;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::Alert(event)))
                if event.rule_id == "alert-recovery" && !event.triggered =>
            {
                cleared_early = true;
            }
            Ok(Ok(AppEvent::SessionData(snapshot))) if snapshot.definition_id == definition.id => {
                break;
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        !cleared_early,
        "recovery must wait for its configured hold time"
    );

    tokio::time::sleep(Duration::from_millis(250)).await;
    manager
        .read_once(&session.id, &definition.id)
        .await
        .expect("recovery read");
    let mut cleared = None;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(AppEvent::Alert(event)))
                if event.rule_id == "alert-recovery" && !event.triggered =>
            {
                cleared = Some(event);
            }
            Ok(Ok(AppEvent::SessionData(snapshot))) if snapshot.definition_id == definition.id => {
                break;
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    let cleared = cleared.expect("recovery event");
    assert_eq!(cleared.recovery_operator, Some(AlertOperator::Lt));
    assert_eq!(cleared.recovery_threshold, Some(300.0));
    assert_eq!(cleared.recovery_delay_ms, Some(200));
    assert_eq!(cleared.description, "电压恢复测试");
    assert_eq!(cleared.severity, AlertSeverity::Warning);

    manager.shutdown_all().await;
    server.abort();
}

#[tokio::test]
async fn restores_sessions_without_connecting() {
    let bus = event_bus();
    let manager = SessionManager::new(bus);
    let mut restore = input(6502);
    restore.auto_connect = false;
    let session = manager
        .create_session(restore)
        .await
        .expect("restore session");
    assert_eq!(session.status, SessionStatus::Disconnected);
    manager.shutdown_all().await;
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
    assert_eq!(session.status, SessionStatus::Disconnected);
    assert_eq!(session.definitions.len(), 1);
    let definition = session.definitions[0].clone();
    assert!(definition.points.is_empty());
    assert_eq!(definition.quantity, 0);

    let error = manager
        .connect(&session.id)
        .await
        .expect_err("empty mapping must not start");
    assert!(error.contains("模拟映射"), "unexpected error: {error}");

    let updated = manager
        .update_definition(
            &session.id,
            &definition.id,
            DefinitionInput {
                name: definition.name.clone(),
                function_code: 0x03,
                slave_id: 1,
                start_address: 0,
                quantity: 2,
                data_type: DataType::U16,
                byte_order: OrderPreset::Abcd,
                byte_pos: None,
                interval_ms: 500,
                enabled: true,
                aliases: BTreeMap::new(),
                points: vec![
                    RegisterPointDto {
                        address: 0,
                        data_type: DataType::U16,
                        byte_order: OrderPreset::Abcd,
                        byte_pos: None,
                        meaning: "温度".to_owned(),
                        unit: "C".to_owned(),
                        scale: 1.0,
                        offset: 0.0,
                        alerts: Vec::new(),
                        simulator: Some(SimulatorPointDto {
                            mode: SimulatorMode::Fixed,
                            refresh_ms: 1000,
                            min: 1000.0,
                            max: 1000.0,
                            enabled: false,
                        }),
                    },
                    RegisterPointDto {
                        address: 1,
                        data_type: DataType::U16,
                        byte_order: OrderPreset::Abcd,
                        byte_pos: None,
                        meaning: "压力".to_owned(),
                        unit: "bar".to_owned(),
                        scale: 1.0,
                        offset: 0.0,
                        alerts: Vec::new(),
                        simulator: Some(SimulatorPointDto {
                            mode: SimulatorMode::Fixed,
                            refresh_ms: 1000,
                            min: 1001.0,
                            max: 1001.0,
                            enabled: false,
                        }),
                    },
                ],
            },
        )
        .await
        .expect("update simulator mapping");
    assert_eq!(updated.points.len(), 2);
    manager.connect(&session.id).await.expect("connect slave");

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
