use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use modbus_core::frame;
use modbus_core::simulator::{Simulator, SlaveUnit};
use modbus_core::{DataType, Framing, OrderPreset};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::event::{AppEvent, event_bus};
use crate::model::{
    CustomWriteInput, DefinitionInput, NewSessionInput, SessionRole, TransportKind,
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
