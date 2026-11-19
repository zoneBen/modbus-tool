use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct ToolTasks {
    tasks: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl ToolTasks {
    pub fn register(&self, task_id: &str) -> Arc<AtomicBool> {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        tasks.insert(task_id.to_owned(), cancel.clone());
        cancel
    }

    pub fn finish(&self, task_id: &str) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        tasks.remove(task_id);
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        let tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match tasks.get(task_id) {
            Some(cancel) => {
                cancel.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub fn cancel_all(&self) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for cancel in tasks.values() {
            cancel.store(true, Ordering::Relaxed);
        }
        tasks.clear();
    }
}
