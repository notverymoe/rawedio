
//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::{cell::UnsafeCell, ops::{Deref, DerefMut}};
use portable_atomic::{AtomicU64, Ordering};

use triomphe::Arc;

use crossbeam_utils::CachePadded;

/// Creates a new pool queue, initializing every slot of capacity with the given function.
/// 
/// See `PoolQueue` for information on the behaviour of the structure.
/// 
/// Returns the producer and consumer pair that can access the pool queue.
pub fn create_pool_queue<T: Send, F>(capacity: usize, init: F) -> (PoolQueueProd<T>, PoolQueueCons<T>) where F : FnMut() -> T {
    let queue = Arc::new(PoolQueue::new(capacity, init));
    (
        PoolQueueProd::from_queue(Arc::clone(&queue)),
        PoolQueueCons::from_queue(Arc::clone(&queue)),
    )
}

/// Creates a new pool queue, initializing every slot of capacity with the default value of the type.
/// 
/// Returns the producer and consumer pair that can access the pool queue.
pub fn create_pool_queue_default<T: Send + Default>(capacity: usize) -> (PoolQueueProd<T>, PoolQueueCons<T>)  {
    create_pool_queue(capacity, T::default)
}

// // Pool Queue // //

/// A pool queue is a simple high-performance lockless fixed-size
/// ring-buffer designed for SPSC communication, where slots are never
/// dropped. This allows for optimization in case where the data isn't
/// consumed by the consumer, as it allows any allocations to be
/// reused by the producer (ie. heap allocations).
struct PoolQueue<T: Send> {
    store: Box<[UnsafeCell<T>]>,
    head: CachePadded<AtomicU64>,
    tail: CachePadded<AtomicU64>,
}

impl<T: Send> PoolQueue<T> {

    fn new<F>(capacity: usize, mut init: F) -> Self where F : FnMut() -> T {
        let mut store = Vec::with_capacity(capacity);
        store.resize_with(capacity, move || UnsafeCell::new(init()));
        let store = store.into_boxed_slice();
        Self{
            store,
            head: CachePadded::new(AtomicU64::new(0)),
            tail: CachePadded::new(AtomicU64::new(0)),
        }
    }

}

// Safety: The structure is immediately wrapped by the Prod and Cons
//     pair that handle properly synchronized access to the Pool Queue.
unsafe impl<T: Send> Sync for PoolQueue<T> {}

// // Producer // //

/// Producer for the pool queue
pub struct PoolQueueProd<T: Send> {
    len: u64,
    queue: Arc<PoolQueue<T>>,
    cached_tail: u64,
}

impl<T: Send> PoolQueueProd<T> {

    /// Tries to obtain mutable access to the current slot.
    /// 
    /// When `PoolQueueSlot` moves out of scope, it will automatically
    /// be sent to the associated `PoolQueueCons`. You can call
    /// `PoolQueueSlot::dismiss` to supress the automatic behaviour
    /// and `PoolQueueSlot::commit` to send it early.
    /// 
    /// `None` when the associated pool queue is full.
    /// 
    pub fn enqueue(&mut self) -> Option<PoolQueueSlot<'_, T>> {
        let head = self.queue.head.load(Ordering::Acquire);
        if head - self.cached_tail == self.len {
            self.cached_tail = self.queue.tail.load(Ordering::Acquire);
            if head - self.cached_tail == self.len {
                return None;
            }
        }

        let idx = (head % self.len) as usize;

        // Safety: The tail will never access the same slot as the
        //     head. Additionally, this function takes the
        //     mutable reference to `self`, meaning we have exclusive
        //     access on this thread.
        let value = unsafe{ &mut *self.queue.store[idx].get() };

        Some(PoolQueueSlot::new(&self.queue.head, value))
    }

    /// Returns if the pool queue still has an associated consumer.
    /// If this if false, it means the queue will never be consumed.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        Arc::strong_count(&self.queue) >= 2
    }

}

impl<T: Send> PoolQueueProd<T> {

    fn from_queue(queue: Arc<PoolQueue<T>>) -> Self {
        Self {
            len: queue.store.len() as u64,
            queue,
            cached_tail: 0,
        }
    }

}

// // Consumer // //

/// Consumer for the pool queue
pub struct PoolQueueCons<T: Send> {
    len: u64,
    queue: Arc<PoolQueue<T>>,
    cached_head: u64,
}

impl<T: Send> PoolQueueCons<T> {

    /// Tries to obtain mutable access to the current slot.
    /// 
    /// When `PoolQueueSlot` moves out of scope, it will automatically
    /// be returned to the associated `PoolQueueProds`. You can call
    /// `PoolQueueSlot::dismiss` to supress the automatic behaviour
    /// and `PoolQueueSlot::commit` to return it early.
    /// 
    /// `None` when the associated pool queue is empty.
    /// 
    pub fn dequeue(&mut self) -> Option<PoolQueueSlot<'_, T>> {
        let tail = self.queue.tail.load(Ordering::Acquire);
        if tail == self.cached_head {
            self.cached_head = self.queue.head.load(Ordering::Acquire);
            if tail == self.cached_head {
                return None;
            }
        }

        let idx = (tail % self.len) as usize;

        // Safety: We ensured that the tail isn't on the same slot as
        //    the head, which means we're not accessing the same slot
        //    across threads. Additionally, this function takes the
        //    mutable reference to `self`, meaning we have exclusive
        //    access on this thread.
        let value = unsafe{ &mut *self.queue.store[idx].get() };

        Some(PoolQueueSlot::new(&self.queue.tail, value))
    }

    /// Returns if the pool queue still has an associated consumer.
    /// If this if false, it means the queue will never be consumed.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        Arc::strong_count(&self.queue) >= 2
    }

}

impl<T: Send> PoolQueueCons<T> {

    fn from_queue(queue: Arc<PoolQueue<T>>) -> Self {
        Self {
            len: queue.store.len() as u64,
            queue,
            cached_head: 0,
        }
    }

}


// // SlotView // //

/// Ensures exclusive mutable access to a slot in the pool queue and
/// then automatically commits it to the next queue (to the Prod/Cons)
/// when dropped, if `PoolQueueSlot::dismiss` isn't called.
pub struct PoolQueueSlot<'a, T> {
    counter: &'a AtomicU64,
    value: &'a mut T,
    done: bool,
}

impl<T: Send> PoolQueueSlot<'_, T> {

    /// Disables the `commit on drop behaviour`
    pub const fn dismiss(&mut self) {
        self.done = true;
    }

    /// Causes the slot to be submitted to the next
    /// queue (to the Prod/Cons)
    pub fn commit(mut self) {
        self.done = false;
        std::mem::drop(self);
    }

}

impl<T> Deref for PoolQueueSlot<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> DerefMut for PoolQueueSlot<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<'a, T> PoolQueueSlot<'a, T> {
    const fn new(counter: &'a AtomicU64, value: &'a mut T) -> Self {
        Self {
            counter,
            value,
            done: false
        }
    }
}

impl<T> Drop for PoolQueueSlot<'_, T> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        if std::mem::replace(&mut self.done, true) { return; }
        self.counter.fetch_add(1, Ordering::Release);
    }
}

// // Test // //

#[cfg(test)]
mod tests {

}