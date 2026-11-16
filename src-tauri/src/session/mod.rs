//! Session lifecycle and command routing.

pub mod actor;
pub mod slave;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use tokio::sync::{RwLock, mpsc, oneshot};
use uuid::Uuid;

use crate::event::{AppEvent, EventBus, emit};
use crate::model::{
    CustomWriteInput, DefinitionDto, DefinitionInput, NewSessionInput, SessionDto, SessionState,
    WorkspaceSnapshotDto,
};

use actor::{Actor, SessionCommand};

#[derive(Clone)]
pub struct SessionHandle {
    pub state: Arc<RwLock<SessionState>>,
    pub commands: mpsc::UnboundedSender<SessionCommand>,
}

pub struct SessionManager {
    sessions: RwLock<HashMap<String, SessionHandle>>,
    bus: EventBus,
    seq: Arc<AtomicU64>,
}

impl SessionManager {
    pub fn new(bus: EventBus) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            bus,
            seq: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn create_session(&self, input: NewSessionInput) -> Result<SessionDto, String> {
        let id = Uuid::new_v4().to_string();
        let config = input.into_config(id.clone())?;
        let state = Arc::new(RwLock::new(SessionState::new(config)));
        let (commands, command_rx) = mpsc::unbounded_channel();
        let actor = Actor::new(
            state.clone(),
            self.bus.clone(),
            command_rx,
            self.seq.clone(),
        );
        tokio::spawn(actor.run());

        let handle = SessionHandle {
            state: state.clone(),
            commands: commands.clone(),
        };
        self.sessions.write().await.insert(id.clone(), handle);

        let (reply, response) = oneshot::channel();
        if commands.send(SessionCommand::Connect { reply }).is_err() {
            self.sessions.write().await.remove(&id);
            return Err("会话已关闭".to_owned());
        }

        match tokio::time::timeout(Duration::from_secs(10), response).await {
            Ok(Ok(Ok(()))) => {
                let dto = state.read().await.to_dto();
                self.emit_workspace().await;
                Ok(dto)
            }
            Ok(Ok(Err(error))) => {
                self.sessions.write().await.remove(&id);
                let _ = commands.send(SessionCommand::Shutdown);
                Err(error)
            }
            Ok(Err(_)) => {
                self.sessions.write().await.remove(&id);
                let _ = commands.send(SessionCommand::Shutdown);
                Err("会话未响应连接请求".to_owned())
            }
            Err(_) => {
                self.sessions.write().await.remove(&id);
                let _ = commands.send(SessionCommand::Shutdown);
                Err("连接超时".to_owned())
            }
        }
    }

    pub async fn workspace(&self) -> WorkspaceSnapshotDto {
        let handles: Vec<SessionHandle> = {
            let sessions = self.sessions.read().await;
            sessions.values().cloned().collect()
        };
        let mut sessions = Vec::with_capacity(handles.len());
        for handle in handles {
            sessions.push(handle.state.read().await.to_dto());
        }
        sessions.sort_by_key(|session| session.started_at);
        WorkspaceSnapshotDto {
            sessions,
            version: 1,
            demo: false,
        }
    }

    pub async fn set_polling(&self, session_id: &str, enabled: bool) -> Result<(), String> {
        self.ask(session_id, |reply| SessionCommand::SetPolling {
            enabled,
            reply,
        })
        .await
    }

    pub async fn read_once(&self, session_id: &str, definition_id: &str) -> Result<(), String> {
        self.ask(session_id, |reply| SessionCommand::ReadOnce {
            definition_id: definition_id.to_owned(),
            reply,
        })
        .await
    }

    pub async fn set_definition_enabled(
        &self,
        session_id: &str,
        definition_id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let result = self
            .ask(session_id, |reply| SessionCommand::SetDefinitionEnabled {
                definition_id: definition_id.to_owned(),
                enabled,
                reply,
            })
            .await;
        if result.is_ok() {
            self.emit_workspace().await;
        }
        result
    }

    pub async fn create_definition(
        &self,
        session_id: &str,
        input: DefinitionInput,
    ) -> Result<DefinitionDto, String> {
        let definition = self
            .ask(session_id, |reply| SessionCommand::CreateDefinition {
                input,
                reply,
            })
            .await?;
        self.emit_workspace().await;
        Ok(definition)
    }

    pub async fn update_definition(
        &self,
        session_id: &str,
        definition_id: &str,
        input: DefinitionInput,
    ) -> Result<DefinitionDto, String> {
        let definition = self
            .ask(session_id, |reply| SessionCommand::UpdateDefinition {
                definition_id: definition_id.to_owned(),
                input,
                reply,
            })
            .await?;
        self.emit_workspace().await;
        Ok(definition)
    }

    pub async fn delete_definition(
        &self,
        session_id: &str,
        definition_id: &str,
    ) -> Result<(), String> {
        self.ask(session_id, |reply| SessionCommand::DeleteDefinition {
            definition_id: definition_id.to_owned(),
            reply,
        })
        .await?;
        self.emit_workspace().await;
        Ok(())
    }

    pub async fn write_register(
        &self,
        session_id: &str,
        definition_id: &str,
        address: u16,
        value: &str,
    ) -> Result<(), String> {
        self.ask(session_id, |reply| SessionCommand::WriteRegister {
            definition_id: definition_id.to_owned(),
            address,
            value: value.to_owned(),
            reply,
        })
        .await
    }

    pub async fn write_custom(
        &self,
        session_id: &str,
        definition_id: Option<&str>,
        input: CustomWriteInput,
    ) -> Result<(), String> {
        self.ask(session_id, |reply| SessionCommand::WriteCustom {
            definition_id: definition_id.map(str::to_owned),
            input,
            reply,
        })
        .await
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<(), String> {
        let result = self
            .ask(session_id, |reply| SessionCommand::Disconnect { reply })
            .await;
        if result.is_ok() {
            self.emit_workspace().await;
        }
        result
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let handle = self
            .sessions
            .write()
            .await
            .remove(session_id)
            .ok_or_else(|| "会话不存在".to_owned())?;
        let _ = handle.commands.send(SessionCommand::Shutdown);
        self.emit_workspace().await;
        Ok(())
    }

    pub async fn shutdown_all(&self) {
        let handles: Vec<SessionHandle> = {
            let mut sessions = self.sessions.write().await;
            sessions.drain().map(|(_, handle)| handle).collect()
        };
        for handle in handles {
            let _ = handle.commands.send(SessionCommand::Shutdown);
        }
    }

    async fn ask<T>(
        &self,
        session_id: &str,
        make_command: impl FnOnce(oneshot::Sender<Result<T, String>>) -> SessionCommand,
    ) -> Result<T, String> {
        let handle = self
            .sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| "会话不存在".to_owned())?;
        let (reply, response) = oneshot::channel();
        handle
            .commands
            .send(make_command(reply))
            .map_err(|_| "会话已关闭".to_owned())?;
        match tokio::time::timeout(Duration::from_secs(30), response).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("会话未返回结果".to_owned()),
            Err(_) => Err("操作超时".to_owned()),
        }
    }

    async fn emit_workspace(&self) {
        emit(
            &self.bus,
            AppEvent::WorkspaceUpdated(self.workspace().await),
        );
    }
}

#[cfg(test)]
mod tests;
