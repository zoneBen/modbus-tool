use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rumqttc::v5::mqttbytes::QoS as V5QoS;
use rumqttc::{
    AsyncClient as V4Client, Event as V4Event, MqttOptions as V4Options, Packet as V4Packet, QoS,
    TlsConfiguration, Transport,
};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::event::{AppEvent, EventBus, emit};

use super::model::{
    MqttBrokerInput, MqttMessageDto, MqttPublishInput, MqttStateDto, MqttSubscribeInput,
    ToolErrorDto, ToolResult,
};

const MAX_PACKET_BYTES: usize = 1024 * 1024;
const MAX_CA_BYTES: u64 = 1024 * 1024;

pub struct MqttManager {
    bus: EventBus,
    brokers: Mutex<HashMap<String, BrokerRuntime>>,
}

#[derive(Clone)]
enum BrokerClient {
    V4(V4Client),
    V5(rumqttc::v5::AsyncClient),
}

struct BrokerRuntime {
    client: BrokerClient,
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl MqttManager {
    pub fn new(bus: EventBus) -> Self {
        Self {
            bus,
            brokers: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start(&self, input: MqttBrokerInput) -> ToolResult<()> {
        let broker_id = input.broker_id.trim().to_owned();
        if broker_id.is_empty() {
            return Err(ToolErrorDto::new("invalidBroker", "Broker id is required"));
        }
        if input.host.trim().is_empty() {
            return Err(ToolErrorDto::new("invalidHost", "Broker host is required"));
        }
        if input.port == 0 {
            return Err(ToolErrorDto::new(
                "invalidPort",
                "Port must be between 1 and 65535",
            ));
        }
        if self.is_running(&broker_id) {
            return Err(ToolErrorDto::new(
                "brokerAlreadyRunning",
                "This broker is already connected",
            ));
        }
        let transport = build_transport(input.tls, input.ca_path.as_deref()).await?;
        let client_id = if input.client_id.trim().is_empty() {
            format!("modbus-tool-{}", &Uuid::new_v4().to_string()[..8])
        } else {
            input.client_id.trim().to_owned()
        };
        let keep_alive = input.keep_alive.clamp(5, 600) as u64;
        let cancel = Arc::new(AtomicBool::new(false));
        let bus = self.bus.clone();
        let broker_id_for_task = broker_id.clone();
        let protocol = input.protocol.trim().to_owned();
        emit_state(&bus, &broker_id, "connecting", None);
        let runtime = if protocol == "5" || protocol == "5.0" || protocol.eq_ignore_ascii_case("v5")
        {
            let mut options =
                rumqttc::v5::MqttOptions::new(client_id, input.host.trim(), input.port);
            options.set_keep_alive(Duration::from_secs(keep_alive));
            options.set_clean_start(input.clean_session);
            options.set_max_packet_size(Some(MAX_PACKET_BYTES as u32));
            options.set_transport(transport);
            if !input.username.trim().is_empty() {
                options.set_credentials(
                    input.username.trim(),
                    input.password.as_deref().unwrap_or_default(),
                );
            }
            let (client, mut eventloop) = rumqttc::v5::AsyncClient::new(options, 64);
            let cancel_for_task = cancel.clone();
            let bus_for_task = bus.clone();
            let handle = tokio::spawn(async move {
                loop {
                    if cancel_for_task.load(Ordering::Relaxed) {
                        break;
                    }
                    match eventloop.poll().await {
                        Ok(rumqttc::v5::Event::Incoming(
                            rumqttc::v5::mqttbytes::v5::Packet::ConnAck(_),
                        )) => {
                            emit_state(&bus_for_task, &broker_id_for_task, "connected", None);
                        }
                        Ok(rumqttc::v5::Event::Incoming(
                            rumqttc::v5::mqttbytes::v5::Packet::Publish(publish),
                        )) => {
                            emit_message(
                                &bus_for_task,
                                &broker_id_for_task,
                                publish.topic,
                                publish.payload,
                                v5_qos_value(publish.qos),
                                publish.retain,
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            emit_state(
                                &bus_for_task,
                                &broker_id_for_task,
                                "reconnecting",
                                Some(error.to_string()),
                            );
                            if cancel_for_task.load(Ordering::Relaxed) {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                        }
                    }
                }
                emit_state(&bus_for_task, &broker_id_for_task, "disconnected", None);
            });
            BrokerRuntime {
                client: BrokerClient::V5(client),
                cancel,
                handle,
            }
        } else {
            let mut options = V4Options::new(client_id, input.host.trim(), input.port);
            options.set_keep_alive(Duration::from_secs(keep_alive));
            options.set_clean_session(input.clean_session);
            options.set_max_packet_size(MAX_PACKET_BYTES, MAX_PACKET_BYTES);
            options.set_transport(transport);
            if !input.username.trim().is_empty() {
                options.set_credentials(
                    input.username.trim(),
                    input.password.as_deref().unwrap_or_default(),
                );
            }
            let (client, mut eventloop) = V4Client::new(options, 64);
            let cancel_for_task = cancel.clone();
            let bus_for_task = bus.clone();
            let handle = tokio::spawn(async move {
                loop {
                    if cancel_for_task.load(Ordering::Relaxed) {
                        break;
                    }
                    match eventloop.poll().await {
                        Ok(V4Event::Incoming(V4Packet::ConnAck(_))) => {
                            emit_state(&bus_for_task, &broker_id_for_task, "connected", None);
                        }
                        Ok(V4Event::Incoming(V4Packet::Publish(publish))) => {
                            emit_message(
                                &bus_for_task,
                                &broker_id_for_task,
                                publish.topic,
                                publish.payload,
                                qos_value(publish.qos),
                                publish.retain,
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            emit_state(
                                &bus_for_task,
                                &broker_id_for_task,
                                "reconnecting",
                                Some(error.to_string()),
                            );
                            if cancel_for_task.load(Ordering::Relaxed) {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                        }
                    }
                }
                emit_state(&bus_for_task, &broker_id_for_task, "disconnected", None);
            });
            BrokerRuntime {
                client: BrokerClient::V4(client),
                cancel,
                handle,
            }
        };
        {
            let mut brokers = self
                .brokers
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if brokers.contains_key(&broker_id) {
                runtime.cancel.store(true, Ordering::Relaxed);
                runtime.handle.abort();
                return Err(ToolErrorDto::new(
                    "brokerAlreadyRunning",
                    "This broker is already connected",
                ));
            }
            brokers.insert(broker_id, runtime);
        }
        Ok(())
    }

    pub async fn publish(&self, input: MqttPublishInput) -> ToolResult<()> {
        let payload = match input.payload_mode.trim().to_ascii_lowercase().as_str() {
            "hex" => decode_hex(&input.payload)?,
            _ => input.payload.into_bytes(),
        };
        if payload.len() > MAX_PACKET_BYTES {
            return Err(ToolErrorDto::new(
                "payloadTooLarge",
                "Payload exceeds the 1 MB limit",
            ));
        }
        if input.topic.trim().is_empty() {
            return Err(ToolErrorDto::new("invalidTopic", "Topic is required"));
        }
        let client = self.client(&input.broker_id)?;
        match client {
            BrokerClient::V4(client) => client
                .publish(input.topic.trim(), qos(input.qos)?, input.retain, payload)
                .await
                .map_err(|error| ToolErrorDto::new("mqttPublishFailed", error.to_string())),
            BrokerClient::V5(client) => client
                .publish(
                    input.topic.trim(),
                    v5_qos(input.qos)?,
                    input.retain,
                    payload,
                )
                .await
                .map_err(|error| ToolErrorDto::new("mqttPublishFailed", error.to_string())),
        }
    }

    pub async fn subscribe(&self, input: MqttSubscribeInput) -> ToolResult<()> {
        if input.topic.trim().is_empty() {
            return Err(ToolErrorDto::new(
                "invalidTopic",
                "Topic filter is required",
            ));
        }
        let client = self.client(&input.broker_id)?;
        match client {
            BrokerClient::V4(client) => client
                .subscribe(input.topic.trim(), qos(input.qos)?)
                .await
                .map_err(|error| ToolErrorDto::new("mqttSubscribeFailed", error.to_string())),
            BrokerClient::V5(client) => client
                .subscribe(input.topic.trim(), v5_qos(input.qos)?)
                .await
                .map_err(|error| ToolErrorDto::new("mqttSubscribeFailed", error.to_string())),
        }
    }

    pub async fn unsubscribe(&self, broker_id: &str, topic: &str) -> ToolResult<()> {
        if topic.trim().is_empty() {
            return Err(ToolErrorDto::new(
                "invalidTopic",
                "Topic filter is required",
            ));
        }
        let client = self.client(broker_id)?;
        match client {
            BrokerClient::V4(client) => client
                .unsubscribe(topic.trim())
                .await
                .map_err(|error| ToolErrorDto::new("mqttUnsubscribeFailed", error.to_string())),
            BrokerClient::V5(client) => client
                .unsubscribe(topic.trim())
                .await
                .map_err(|error| ToolErrorDto::new("mqttUnsubscribeFailed", error.to_string())),
        }
    }

    pub async fn stop(&self, broker_id: &str) -> bool {
        let runtime = {
            let mut brokers = self
                .brokers
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            brokers.remove(broker_id)
        };
        let Some(BrokerRuntime {
            client,
            cancel,
            handle,
        }) = runtime
        else {
            return false;
        };
        cancel.store(true, Ordering::Relaxed);
        match &client {
            BrokerClient::V4(client) => {
                let _ = client.disconnect().await;
            }
            BrokerClient::V5(client) => {
                let _ = client.disconnect().await;
            }
        }
        let abort = handle.abort_handle();
        if tokio::time::timeout(Duration::from_millis(400), handle)
            .await
            .is_err()
        {
            abort.abort();
        }
        emit_state(&self.bus, broker_id, "disconnected", None);
        true
    }

    pub async fn stop_all(&self) {
        let runtimes = {
            let mut brokers = self
                .brokers
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            std::mem::take(&mut *brokers)
        };
        for (_, runtime) in runtimes {
            runtime.cancel.store(true, Ordering::Relaxed);
            let abort = runtime.handle.abort_handle();
            match &runtime.client {
                BrokerClient::V4(client) => {
                    let _ = client.disconnect().await;
                }
                BrokerClient::V5(client) => {
                    let _ = client.disconnect().await;
                }
            }
            if tokio::time::timeout(Duration::from_millis(300), runtime.handle)
                .await
                .is_err()
            {
                abort.abort();
            }
        }
    }

    fn is_running(&self, broker_id: &str) -> bool {
        self.brokers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .contains_key(broker_id)
    }

    fn client(&self, broker_id: &str) -> ToolResult<BrokerClient> {
        self.brokers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(broker_id)
            .map(|runtime| runtime.client.clone())
            .ok_or_else(|| ToolErrorDto::new("brokerNotRunning", "Broker is not connected"))
    }
}

async fn build_transport(tls: bool, ca_path: Option<&str>) -> ToolResult<Transport> {
    if !tls {
        return Ok(Transport::tcp());
    }
    let ca_path = ca_path.map(str::trim).filter(|path| !path.is_empty());
    if let Some(path) = ca_path {
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|error| ToolErrorDto::new("caReadFailed", error.to_string()))?;
        if metadata.len() > MAX_CA_BYTES {
            return Err(ToolErrorDto::new("caReadFailed", "CA file is too large"));
        }
        let ca = tokio::fs::read(path)
            .await
            .map_err(|error| ToolErrorDto::new("caReadFailed", error.to_string()))?;
        Ok(Transport::tls_with_config(TlsConfiguration::SimpleNative {
            ca,
            client_auth: None,
        }))
    } else {
        Ok(Transport::tls_with_config(TlsConfiguration::Native))
    }
}

fn qos(value: u8) -> ToolResult<QoS> {
    match value {
        0 => Ok(QoS::AtMostOnce),
        1 => Ok(QoS::AtLeastOnce),
        2 => Ok(QoS::ExactlyOnce),
        _ => Err(ToolErrorDto::new("invalidQos", "QoS must be 0, 1 or 2")),
    }
}

fn v5_qos(value: u8) -> ToolResult<V5QoS> {
    match value {
        0 => Ok(V5QoS::AtMostOnce),
        1 => Ok(V5QoS::AtLeastOnce),
        2 => Ok(V5QoS::ExactlyOnce),
        _ => Err(ToolErrorDto::new("invalidQos", "QoS must be 0, 1 or 2")),
    }
}

fn qos_value(value: QoS) -> u8 {
    match value {
        QoS::AtMostOnce => 0,
        QoS::AtLeastOnce => 1,
        QoS::ExactlyOnce => 2,
    }
}

fn v5_qos_value(value: V5QoS) -> u8 {
    match value {
        V5QoS::AtMostOnce => 0,
        V5QoS::AtLeastOnce => 1,
        V5QoS::ExactlyOnce => 2,
    }
}

fn decode_hex(text: &str) -> ToolResult<Vec<u8>> {
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let cleaned = compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
        .unwrap_or(&compact);
    if !cleaned.len().is_multiple_of(2) {
        return Err(ToolErrorDto::new(
            "invalidHex",
            "HEX payload must contain an even number of digits",
        ));
    }
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    let chars: Vec<char> = cleaned.chars().collect();
    for pair in chars.chunks(2) {
        let value = pair.iter().collect::<String>();
        let byte = u8::from_str_radix(&value, 16)
            .map_err(|_| ToolErrorDto::new("invalidHex", format!("Invalid HEX byte: {value}")))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn emit_state(bus: &EventBus, broker_id: &str, status: &str, detail: Option<String>) {
    emit(
        bus,
        AppEvent::MqttState(MqttStateDto {
            broker_id: broker_id.to_owned(),
            status: status.to_owned(),
            detail,
            timestamp_ms: now_ms(),
        }),
    );
}

fn emit_message(
    bus: &EventBus,
    broker_id: &str,
    topic: impl AsRef<[u8]>,
    payload: impl AsRef<[u8]>,
    qos: u8,
    retain: bool,
) {
    let topic = String::from_utf8_lossy(topic.as_ref()).into_owned();
    let payload = payload.as_ref();
    emit(
        bus,
        AppEvent::MqttMessage(MqttMessageDto {
            broker_id: broker_id.to_owned(),
            topic,
            payload_text: String::from_utf8_lossy(payload).into_owned(),
            payload_hex: payload
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" "),
            payload_bytes: payload.len(),
            qos,
            retain,
            timestamp_ms: now_ms(),
        }),
    );
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_hex_payloads() {
        assert_eq!(decode_hex("01 0A ff").expect("hex"), vec![1, 10, 255]);
        assert_eq!(decode_hex("0x4D4F").expect("hex"), vec![0x4D, 0x4F]);
        assert!(decode_hex("ABC").is_err());
        assert!(decode_hex("GG").is_err());
    }

    #[test]
    fn validates_qos() {
        assert!(qos(0).is_ok());
        assert!(qos(2).is_ok());
        assert!(qos(3).is_err());
    }

    #[test]
    fn emits_full_payload_without_truncation() {
        let bus = crate::event::event_bus();
        let mut receiver = bus.subscribe();
        let payload = vec![b'a'; 32 * 1024];
        emit_message(&bus, "broker", "demo/large", &payload, 1, false);
        match receiver.try_recv().expect("mqtt message") {
            crate::event::AppEvent::MqttMessage(message) => {
                assert_eq!(message.payload_bytes, payload.len());
                assert_eq!(message.payload_text.len(), payload.len());
                assert_eq!(message.payload_hex.split(' ').count(), payload.len());
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
