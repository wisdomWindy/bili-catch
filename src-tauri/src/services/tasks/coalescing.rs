use std::{collections::HashMap, sync::Mutex};

use time::{Duration, OffsetDateTime};

#[derive(Clone, Copy)]
struct ProgressWindow {
    last_flush_at: OffsetDateTime,
    dirty: bool,
}

#[derive(Default)]
pub(super) struct ProgressCoalescer {
    windows: Mutex<HashMap<String, ProgressWindow>>,
}

impl ProgressCoalescer {
    pub(super) fn should_flush(&self, task_id: &str, now: OffsetDateTime) -> bool {
        self.windows
            .lock()
            .expect("progress coalescer lock poisoned")
            .get(task_id)
            .is_none_or(|window| now - window.last_flush_at >= Duration::seconds(1))
    }

    pub(super) fn mark_dirty(&self, task_id: &str) {
        if let Some(window) = self
            .windows
            .lock()
            .expect("progress coalescer lock poisoned")
            .get_mut(task_id)
        {
            window.dirty = true;
        }
    }

    pub(super) fn mark_flushed(&self, task_id: &str, now: OffsetDateTime) {
        self.windows
            .lock()
            .expect("progress coalescer lock poisoned")
            .insert(
                task_id.to_owned(),
                ProgressWindow {
                    last_flush_at: now,
                    dirty: false,
                },
            );
    }

    pub(super) fn is_dirty(&self, task_id: &str) -> bool {
        self.windows
            .lock()
            .expect("progress coalescer lock poisoned")
            .get(task_id)
            .is_some_and(|window| window.dirty)
    }

    pub(super) fn clear(&self, task_id: &str) {
        self.windows
            .lock()
            .expect("progress coalescer lock poisoned")
            .remove(task_id);
    }
}
