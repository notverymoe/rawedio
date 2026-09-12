//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Provides synchronization containers for cross-thread communication

mod pool_queue;
pub use pool_queue::{create_pool_queue, create_pool_queue_default, PoolQueueProd, PoolQueueCons, PoolQueueSlot};