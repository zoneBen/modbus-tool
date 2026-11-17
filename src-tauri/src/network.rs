//! Standalone TCP/UDP debug assistant: servers, clients and a TCP relay.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot, watch};

use crate::event::{AppEvent, EventBus, emit};
use crate::model::{
    NetworkDataDto, NetworkMode, NetworkStartInput, NetworkStateDto, RawSendInput, now_ms,
};

pub struct NetworkManager {
    bus: EventBus,
    active: RwLock<Option<NetworkHandle>>,
}

struct NetworkHandle {
    stop: watch::Sender<bool>,
    commands: mpsc::UnboundedSender<NetworkCommand>,
    task: tokio::task::JoinHandle<()>,
}

struct NetworkCommand {
    data: Vec<u8>,
    reply: oneshot::Sender<Result<usize, String>>,
}

impl NetworkManager {
    pub fn new(bus: EventBus) -> Self {
        Self {
            bus,
            active: RwLock::new(None),
        }
    }

    pub async fn start(&self, input: NetworkStartInput) -> Result<NetworkStateDto, String> {
        let _ = self.stop().await;
        validate_input(&input)?;
        let mode = input.mode;
        let endpoint = endpoint_for(&input);
        let (stop_tx, stop_rx) = watch::channel(false);
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();
        let bus = self.bus.clone();
        let task_input = input.clone();
        let task = tokio::spawn(async move {
            let result = run_mode(task_input, bus.clone(), command_rx, stop_rx, ready_tx).await;
            let error = result.err();
            emit(
                &bus,
                AppEvent::NetworkState(NetworkStateDto {
                    active: false,
                    mode: Some(mode.label().to_owned()),
                    endpoint: Some(endpoint),
                    clients: 0,
                    error,
                }),
            );
        });

        match tokio::time::timeout(Duration::from_secs(6), ready_rx).await {
            Ok(Ok(Ok(state))) => {
                *self.active.write().await = Some(NetworkHandle {
                    stop: stop_tx,
                    commands: command_tx,
                    task,
                });
                Ok(state)
            }
            Ok(Ok(Err(error))) => {
                let _ = stop_tx.send(true);
                let _ = task.await;
                Err(error)
            }
            Ok(Err(_)) => {
                let _ = stop_tx.send(true);
                let _ = task.await;
                Err("网络助手启动失败".to_owned())
            }
            Err(_) => {
                let _ = stop_tx.send(true);
                task.abort();
                Err("网络助手启动超时".to_owned())
            }
        }
    }

    pub async fn stop(&self) -> Result<(), String> {
        let handle = self.active.write().await.take();
        if let Some(handle) = handle {
            let _ = handle.stop.send(true);
            let _ = tokio::time::timeout(Duration::from_secs(2), handle.task).await;
        }
        Ok(())
    }

    pub async fn send(&self, input: RawSendInput) -> Result<usize, String> {
        let payload = input.encode()?;
        let (reply_tx, reply_rx) = oneshot::channel();
        let command = NetworkCommand {
            data: payload,
            reply: reply_tx,
        };
        let sender = {
            let active = self.active.read().await;
            active
                .as_ref()
                .map(|handle| handle.commands.clone())
                .ok_or_else(|| "网络助手尚未启动".to_owned())?
        };
        sender
            .send(command)
            .map_err(|_| "网络助手连接已关闭".to_owned())?;
        match tokio::time::timeout(Duration::from_secs(3), reply_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("网络助手未返回发送结果".to_owned()),
            Err(_) => Err("网络助手发送超时".to_owned()),
        }
    }
}

fn validate_input(input: &NetworkStartInput) -> Result<(), String> {
    match input.mode {
        NetworkMode::TcpServer | NetworkMode::UdpServer => {
            if input.local_port == 0 {
                return Err("监听端口需在 1–65535 之间".to_owned());
            }
        }
        NetworkMode::TcpClient | NetworkMode::UdpClient => {
            if input.remote_host.as_deref().unwrap_or("").trim().is_empty() {
                return Err("请输入远端主机地址".to_owned());
            }
            if input.remote_port.unwrap_or(0) == 0 {
                return Err("远端端口需在 1–65535 之间".to_owned());
            }
        }
        NetworkMode::TcpRelay => {
            if input.local_port == 0 || input.remote_port.unwrap_or(0) == 0 {
                return Err("本地监听端口和远端端口都需在 1–65535 之间".to_owned());
            }
            if input.remote_host.as_deref().unwrap_or("").trim().is_empty() {
                return Err("请输入上游 TCP 服务地址".to_owned());
            }
        }
    }
    Ok(())
}

fn endpoint_for(input: &NetworkStartInput) -> String {
    let local_host = if input.local_host.trim().is_empty() {
        "0.0.0.0"
    } else {
        input.local_host.trim()
    };
    match input.mode {
        NetworkMode::TcpServer | NetworkMode::UdpServer => {
            format!("{local_host}:{}", input.local_port)
        }
        NetworkMode::TcpClient | NetworkMode::UdpClient => format!(
            "{}:{}",
            input.remote_host.as_deref().unwrap_or(""),
            input.remote_port.unwrap_or(0)
        ),
        NetworkMode::TcpRelay => format!(
            "{local_host}:{} → {}:{}",
            input.local_port,
            input.remote_host.as_deref().unwrap_or(""),
            input.remote_port.unwrap_or(0)
        ),
    }
}

fn started_state(mode: NetworkMode, endpoint: &str) -> NetworkStateDto {
    NetworkStateDto {
        active: true,
        mode: Some(mode.label().to_owned()),
        endpoint: Some(endpoint.to_owned()),
        clients: 0,
        error: None,
    }
}

fn emit_state(
    bus: &EventBus,
    mode: NetworkMode,
    endpoint: &str,
    clients: u32,
    error: Option<String>,
) {
    emit(
        bus,
        AppEvent::NetworkState(NetworkStateDto {
            active: error.is_none(),
            mode: Some(mode.label().to_owned()),
            endpoint: Some(endpoint.to_owned()),
            clients,
            error,
        }),
    );
}

fn emit_data(bus: &EventBus, direction: &str, data: &[u8], connection: &str) {
    emit(
        bus,
        AppEvent::NetworkData(NetworkDataDto {
            direction: direction.to_owned(),
            data: data.to_vec(),
            connection: connection.to_owned(),
            timestamp: now_ms(),
        }),
    );
}

async fn run_mode(
    input: NetworkStartInput,
    bus: EventBus,
    commands: mpsc::UnboundedReceiver<NetworkCommand>,
    stop_rx: watch::Receiver<bool>,
    ready_tx: oneshot::Sender<Result<NetworkStateDto, String>>,
) -> Result<(), String> {
    match input.mode {
        NetworkMode::TcpServer => run_tcp_server(input, bus, commands, stop_rx, ready_tx).await,
        NetworkMode::TcpClient => run_tcp_client(input, bus, commands, stop_rx, ready_tx).await,
        NetworkMode::UdpServer => run_udp_server(input, bus, commands, stop_rx, ready_tx).await,
        NetworkMode::UdpClient => run_udp_client(input, bus, commands, stop_rx, ready_tx).await,
        NetworkMode::TcpRelay => run_tcp_relay(input, bus, commands, stop_rx, ready_tx).await,
    }
}

async fn run_tcp_server(
    input: NetworkStartInput,
    bus: EventBus,
    mut commands: mpsc::UnboundedReceiver<NetworkCommand>,
    mut stop_rx: watch::Receiver<bool>,
    ready_tx: oneshot::Sender<Result<NetworkStateDto, String>>,
) -> Result<(), String> {
    let host = if input.local_host.trim().is_empty() {
        "0.0.0.0"
    } else {
        input.local_host.trim()
    };
    let listener = TcpListener::bind((host, input.local_port))
        .await
        .map_err(|error| format!("无法监听 {host}:{}：{error}", input.local_port))?;
    let endpoint = endpoint_for(&input);
    let mode = NetworkMode::TcpServer;
    let (outgoing, _) = broadcast::channel::<Vec<u8>>(128);
    let clients = Arc::new(AtomicU32::new(0));
    let state = started_state(mode, &endpoint);
    emit(&bus, AppEvent::NetworkState(state.clone()));
    let _ = ready_tx.send(Ok(state));

    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            command = commands.recv() => {
                let Some(command) = command else { break };
                let data = command.data;
                match outgoing.send(data.clone()) {
                    Ok(count) => {
                        emit_data(&bus, "tx", &data, "all clients");
                        let _ = command.reply.send(Ok(count));
                    }
                    Err(_) => {
                        let _ = command
                            .reply
                            .send(Err("当前没有已连接的客户端".to_owned()));
                    }
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((socket, peer)) => {
                        let count = clients.fetch_add(1, Ordering::Relaxed) + 1;
                        emit_state(&bus, mode, &endpoint, count, None);
                        tokio::spawn(tcp_server_client(
                            socket,
                            peer.to_string(),
                            outgoing.subscribe(),
                            bus.clone(),
                            mode,
                            endpoint.clone(),
                            clients.clone(),
                            stop_rx.clone(),
                        ));
                    }
                    Err(error) => return Err(format!("接受 TCP 连接失败：{error}")),
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn tcp_server_client(
    socket: TcpStream,
    peer: String,
    mut outgoing: broadcast::Receiver<Vec<u8>>,
    bus: EventBus,
    mode: NetworkMode,
    endpoint: String,
    clients: Arc<AtomicU32>,
    mut stop_rx: watch::Receiver<bool>,
) {
    let (mut reader, mut writer) = socket.into_split();
    let mut buffer = [0u8; 4096];
    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            data = outgoing.recv() => {
                match data {
                    Ok(data) => {
                        if writer.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
            read = reader.read(&mut buffer) => {
                match read {
                    Ok(0) | Err(_) => break,
                    Ok(count) => emit_data(&bus, "rx", &buffer[..count], &peer),
                }
            }
        }
    }
    if !*stop_rx.borrow() {
        let count = clients.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
        emit_state(&bus, mode, &endpoint, count, None);
    }
}

async fn run_tcp_client(
    input: NetworkStartInput,
    bus: EventBus,
    mut commands: mpsc::UnboundedReceiver<NetworkCommand>,
    mut stop_rx: watch::Receiver<bool>,
    ready_tx: oneshot::Sender<Result<NetworkStateDto, String>>,
) -> Result<(), String> {
    let host = input.remote_host.as_deref().unwrap_or("").trim();
    let port = input.remote_port.unwrap_or(0);
    let stream = TcpStream::connect((host, port))
        .await
        .map_err(|error| format!("连接 {host}:{port} 失败：{error}"))?;
    let endpoint = endpoint_for(&input);
    let mode = NetworkMode::TcpClient;
    let state = started_state(mode, &endpoint);
    emit(&bus, AppEvent::NetworkState(state.clone()));
    let _ = ready_tx.send(Ok(state));
    let (mut reader, mut writer) = stream.into_split();
    let mut buffer = [0u8; 4096];
    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            command = commands.recv() => {
                let Some(command) = command else { break };
                if writer.write_all(&command.data).await.is_err() {
                    let _ = command.reply.send(Err("发送失败，连接已关闭".to_owned()));
                    break;
                }
                emit_data(&bus, "tx", &command.data, &endpoint);
                let _ = command.reply.send(Ok(1));
            }
            read = reader.read(&mut buffer) => {
                match read {
                    Ok(0) => break,
                    Ok(count) => emit_data(&bus, "rx", &buffer[..count], &endpoint),
                    Err(error) => return Err(format!("TCP 连接读取失败：{error}")),
                }
            }
        }
    }
    Ok(())
}

async fn run_udp_server(
    input: NetworkStartInput,
    bus: EventBus,
    mut commands: mpsc::UnboundedReceiver<NetworkCommand>,
    mut stop_rx: watch::Receiver<bool>,
    ready_tx: oneshot::Sender<Result<NetworkStateDto, String>>,
) -> Result<(), String> {
    let host = if input.local_host.trim().is_empty() {
        "0.0.0.0"
    } else {
        input.local_host.trim()
    };
    let socket = UdpSocket::bind((host, input.local_port))
        .await
        .map_err(|error| format!("无法监听 UDP {host}:{}：{error}", input.local_port))?;
    let endpoint = endpoint_for(&input);
    let mode = NetworkMode::UdpServer;
    let state = started_state(mode, &endpoint);
    emit(&bus, AppEvent::NetworkState(state.clone()));
    let _ = ready_tx.send(Ok(state));
    let mut last_peer: Option<std::net::SocketAddr> = None;
    let mut buffer = vec![0u8; 65_535];
    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            command = commands.recv() => {
                let Some(command) = command else { break };
                let Some(peer) = last_peer else {
                    let _ = command.reply.send(Err("尚未收到客户端数据，无法回复".to_owned()));
                    continue;
                };
                match socket.send_to(&command.data, peer).await {
                    Ok(_) => {
                        emit_data(&bus, "tx", &command.data, &peer.to_string());
                        let _ = command.reply.send(Ok(1));
                    }
                    Err(error) => {
                        let _ = command.reply.send(Err(format!("UDP 发送失败：{error}")));
                    }
                }
            }
            received = socket.recv_from(&mut buffer) => {
                match received {
                    Ok((count, peer)) => {
                        last_peer = Some(peer);
                        emit_data(&bus, "rx", &buffer[..count], &peer.to_string());
                    }
                    Err(error) => return Err(format!("UDP 接收失败：{error}")),
                }
            }
        }
    }
    Ok(())
}

async fn run_udp_client(
    input: NetworkStartInput,
    bus: EventBus,
    mut commands: mpsc::UnboundedReceiver<NetworkCommand>,
    mut stop_rx: watch::Receiver<bool>,
    ready_tx: oneshot::Sender<Result<NetworkStateDto, String>>,
) -> Result<(), String> {
    let remote_host = input.remote_host.as_deref().unwrap_or("").trim();
    let remote_port = input.remote_port.unwrap_or(0);
    let local_host = if input.local_host.trim().is_empty() {
        "0.0.0.0"
    } else {
        input.local_host.trim()
    };
    let socket = UdpSocket::bind((local_host, input.local_port))
        .await
        .map_err(|error| format!("绑定本地 UDP 端口失败：{error}"))?;
    socket
        .connect((remote_host, remote_port))
        .await
        .map_err(|error| format!("连接远端 UDP 失败：{error}"))?;
    let endpoint = endpoint_for(&input);
    let mode = NetworkMode::UdpClient;
    let state = started_state(mode, &endpoint);
    emit(&bus, AppEvent::NetworkState(state.clone()));
    let _ = ready_tx.send(Ok(state));
    let mut buffer = vec![0u8; 65_535];
    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            command = commands.recv() => {
                let Some(command) = command else { break };
                match socket.send(&command.data).await {
                    Ok(_) => {
                        emit_data(&bus, "tx", &command.data, &endpoint);
                        let _ = command.reply.send(Ok(1));
                    }
                    Err(error) => {
                        let _ = command.reply.send(Err(format!("UDP 发送失败：{error}")));
                    }
                }
            }
            received = socket.recv(&mut buffer) => {
                match received {
                    Ok(count) => emit_data(&bus, "rx", &buffer[..count], &endpoint),
                    Err(error) => return Err(format!("UDP 接收失败：{error}")),
                }
            }
        }
    }
    Ok(())
}

async fn run_tcp_relay(
    input: NetworkStartInput,
    bus: EventBus,
    mut commands: mpsc::UnboundedReceiver<NetworkCommand>,
    mut stop_rx: watch::Receiver<bool>,
    ready_tx: oneshot::Sender<Result<NetworkStateDto, String>>,
) -> Result<(), String> {
    let remote_host = input.remote_host.as_deref().unwrap_or("").trim();
    let remote_port = input.remote_port.unwrap_or(0);
    let upstream = TcpStream::connect((remote_host, remote_port))
        .await
        .map_err(|error| format!("连接上游 {remote_host}:{remote_port} 失败：{error}"))?;
    let local_host = if input.local_host.trim().is_empty() {
        "0.0.0.0"
    } else {
        input.local_host.trim()
    };
    let listener = TcpListener::bind((local_host, input.local_port))
        .await
        .map_err(|error| format!("无法监听 {local_host}:{}：{error}", input.local_port))?;
    let endpoint = endpoint_for(&input);
    let mode = NetworkMode::TcpRelay;
    let state = started_state(mode, &endpoint);
    emit(&bus, AppEvent::NetworkState(state.clone()));
    let _ = ready_tx.send(Ok(state));

    let mut upstream = upstream;
    let client = loop {
        tokio::select! {
            _ = stop_rx.changed() => return Ok(()),
            command = commands.recv() => {
                let Some(command) = command else { return Ok(()) };
                if upstream.write_all(&command.data).await.is_err() {
                    let _ = command.reply.send(Err("上游连接已关闭".to_owned()));
                    return Ok(());
                }
                emit_data(&bus, "tx", &command.data, "upstream");
                let _ = command.reply.send(Ok(1));
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((socket, _peer)) => break socket,
                    Err(error) => return Err(format!("接受本地客户端失败：{error}")),
                }
            }
        }
    };
    emit_state(&bus, mode, &endpoint, 1, None);
    let (mut local_read, mut local_write) = client.into_split();
    let (mut upstream_read, mut upstream_write) = upstream.into_split();
    let mut local_buffer = [0u8; 8192];
    let mut upstream_buffer = [0u8; 8192];
    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            command = commands.recv() => {
                let Some(command) = command else { break };
                if upstream_write.write_all(&command.data).await.is_err() {
                    let _ = command.reply.send(Err("上游连接已关闭".to_owned()));
                    break;
                }
                emit_data(&bus, "tx", &command.data, "upstream");
                let _ = command.reply.send(Ok(1));
            }
            read = local_read.read(&mut local_buffer) => {
                match read {
                    Ok(0) => break,
                    Ok(count) => {
                        emit_data(&bus, "rx", &local_buffer[..count], "local client");
                        if upstream_write.write_all(&local_buffer[..count]).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => return Err(format!("读取本地客户端失败：{error}")),
                }
            }
            read = upstream_read.read(&mut upstream_buffer) => {
                match read {
                    Ok(0) => break,
                    Ok(count) => {
                        emit_data(&bus, "rx", &upstream_buffer[..count], "upstream");
                        if local_write.write_all(&upstream_buffer[..count]).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => return Err(format!("读取上游响应失败：{error}")),
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AppEvent, event_bus};
    use crate::model::RawSendMode;

    fn server_input(port: u16) -> NetworkStartInput {
        NetworkStartInput {
            mode: NetworkMode::TcpServer,
            local_host: "127.0.0.1".to_owned(),
            local_port: port,
            remote_host: None,
            remote_port: None,
        }
    }

    #[test]
    fn rejects_incomplete_client_and_relay_configuration() {
        let client = NetworkStartInput {
            mode: NetworkMode::TcpClient,
            local_host: String::new(),
            local_port: 0,
            remote_host: None,
            remote_port: Some(502),
        };
        assert!(validate_input(&client).is_err());

        let relay = NetworkStartInput {
            mode: NetworkMode::TcpRelay,
            local_host: "0.0.0.0".to_owned(),
            local_port: 0,
            remote_host: Some("127.0.0.1".to_owned()),
            remote_port: Some(502),
        };
        assert!(validate_input(&relay).is_err());
    }

    #[tokio::test]
    async fn tcp_server_accepts_a_client_and_sends_bytes() {
        let port = {
            let probe = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            probe.local_addr().unwrap().port()
        };
        let bus = event_bus();
        let mut events = bus.subscribe();
        let manager = NetworkManager::new(bus);
        let state = manager.start(server_input(port)).await.unwrap();
        assert!(state.active);

        let mut client = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let accepted = tokio::time::timeout(Duration::from_secs(2), async {
            while let Ok(event) = events.recv().await {
                if let AppEvent::NetworkState(state) = event
                    && state.active
                    && state.clients == 1
                {
                    return;
                }
            }
        })
        .await;
        assert!(accepted.is_ok(), "client connection was not accepted");

        let sent = manager
            .send(RawSendInput {
                data: "hello".to_owned(),
                mode: RawSendMode::Ascii,
                append_cr: false,
                append_lf: false,
            })
            .await
            .unwrap();
        assert_eq!(sent, 1);

        let mut buffer = [0u8; 5];
        tokio::time::timeout(Duration::from_secs(2), client.read_exact(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buffer, b"hello");

        manager.stop().await.unwrap();
    }
}
