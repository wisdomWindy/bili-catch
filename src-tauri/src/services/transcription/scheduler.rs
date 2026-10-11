use std::collections::{HashSet, VecDeque};

pub fn effective_capacity(configured: usize, active_downloads: usize) -> usize {
    let configured = configured.clamp(1, 3);
    if active_downloads > 0 {
        1
    } else {
        configured
    }
}

#[derive(Debug, Default)]
pub struct FairPendingQueue {
    pending: VecDeque<String>,
    active: HashSet<String>,
}

impl FairPendingQueue {
    pub fn enqueue(&mut self, task_id: impl Into<String>) {
        let task_id = task_id.into();
        if !self.active.contains(&task_id) && !self.pending.contains(&task_id) {
            self.pending.push_back(task_id);
        }
    }

    pub fn acquire(&mut self, capacity: usize) -> Option<String> {
        if self.active.len() >= capacity.max(1) {
            return None;
        }
        let task_id = self.pending.pop_front()?;
        self.active.insert(task_id.clone());
        Some(task_id)
    }

    pub fn release(&mut self, task_id: &str) -> bool {
        self.active.remove(task_id)
    }

    pub fn remove(&mut self, task_id: &str) {
        self.pending.retain(|pending| pending != task_id);
        self.active.remove(task_id);
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn active_len(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
mod tests {
    use super::FairPendingQueue;

    #[test]
    fn queue_is_fifo_deduplicated_and_capacity_bounded() {
        let mut queue = FairPendingQueue::default();
        queue.enqueue("one");
        queue.enqueue("two");
        queue.enqueue("one");
        assert_eq!(queue.pending_len(), 2);
        assert_eq!(queue.acquire(1).as_deref(), Some("one"));
        assert!(queue.acquire(1).is_none());
        assert!(queue.release("one"));
        assert_eq!(queue.acquire(1).as_deref(), Some("two"));
        assert_eq!(queue.active_len(), 1);
    }
}
