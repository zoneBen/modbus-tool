use std::sync::Arc;

use crate::network::NetworkManager;
use crate::scan::ScanManager;
use crate::session::SessionManager;

pub struct AppState {
    pub manager: Arc<SessionManager>,
    pub scans: Arc<ScanManager>,
    pub network: Arc<NetworkManager>,
    pub tools: Arc<crate::tools::ToolsManager>,
    pub fileserver: Arc<crate::tools::FileServerManager>,
    pub mqtt: Arc<crate::tools::MqttManager>,
    pub ssh: Arc<crate::tools::SshSessionManager>,
}
