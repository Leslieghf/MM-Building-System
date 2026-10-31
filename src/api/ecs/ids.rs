//! Host-local identifiers shared by ECS request constructors.
use std::sync::atomic::AtomicU64;

pub(crate) static NEXT_INTENT_ID: AtomicU64 = AtomicU64::new(1);
