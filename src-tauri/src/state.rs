use std::sync::Arc;

use crate::scan::ScanManager;
use crate::session::SessionManager;

pub struct AppState {
    pub manager: Arc<SessionManager>,
    pub scans: Arc<ScanManager>,
}
