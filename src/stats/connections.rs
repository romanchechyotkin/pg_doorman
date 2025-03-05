use once_cell::sync::Lazy;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub static TOTAL_CONNECTION_COUNTER: Lazy<Arc<AtomicUsize>> =
    Lazy::new(|| Arc::new(AtomicUsize::new(0)));
pub static TLS_CONNECTION_COUNTER: Lazy<Arc<AtomicUsize>> =
    Lazy::new(|| Arc::new(AtomicUsize::new(0)));
pub static PLAIN_CONNECTION_COUNTER: Lazy<Arc<AtomicUsize>> =
    Lazy::new(|| Arc::new(AtomicUsize::new(0)));
pub static CANCEL_CONNECTION_COUNTER: Lazy<Arc<AtomicUsize>> =
    Lazy::new(|| Arc::new(AtomicUsize::new(0)));
