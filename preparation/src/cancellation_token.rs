use std::sync::{atomic, Arc};

#[derive(Clone, Debug)]
pub struct CancellationToken {
    is_cancelled: Arc<atomic::AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            is_cancelled: Arc::new(atomic::AtomicBool::new(false)),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(atomic::Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn cancel(&self) {
        self.is_cancelled.store(true, atomic::Ordering::SeqCst);
    }
}
