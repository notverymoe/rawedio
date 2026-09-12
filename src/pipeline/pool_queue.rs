
//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::{cell::UnsafeCell, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use crossbeam_utils::CachePadded;

pub fn create_pool_queue<T: Send, F>(capacity: usize, init: F) -> (Prod<T>, Cons<T>) where F : FnMut() -> T {
    let queue = Arc::new(PoolQueue::new(capacity, init));
    (
        Prod::from_queue(Arc::clone(&queue)),
        Cons::from_queue(Arc::clone(&queue)),
    )
}

pub fn create_pool_queue_default<T: Send + Default>(capacity: usize) -> (Prod<T>, Cons<T>)  {
    create_pool_queue(capacity, T::default)
}

// // Pool Queue // //

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

#[allow(unsafe_code)]
unsafe impl<T: Send> Sync for PoolQueue<T> {}

// // Producer // //

pub struct Prod<T: Send> {
    len: u64,
    queue: Arc<PoolQueue<T>>,
    cached_tail: u64,
}

impl<T: Send> Prod<T> {

    pub fn enqueue(&mut self) -> Option<SlotView<'_, T>> {
        let head = self.queue.head.load(Ordering::Acquire);
        if head - self.cached_tail == self.len {
            self.cached_tail = self.queue.tail.load(Ordering::Acquire);
            if head - self.cached_tail == self.len {
                return None;
            }
        }

        let idx = (head % self.len) as usize;

        #[allow(unsafe_code)]
        let value = unsafe{ &mut *self.queue.store[idx].get() };

        Some(SlotView::new(&self.queue.head, value))
    }

}

impl<T: Send> Prod<T> {

    fn from_queue(queue: Arc<PoolQueue<T>>) -> Self {
        Self {
            len: queue.store.len() as u64,
            queue,
            cached_tail: 0,
        }
    }

}

// // Consumer // //

pub struct Cons<T: Send> {
    len: u64,
    queue: Arc<PoolQueue<T>>,
    cached_head: u64,
}

impl<T: Send> Cons<T> {

    pub fn dequeue(&mut self) -> Option<SlotView<'_, T>> {
        let tail = self.queue.tail.load(Ordering::Acquire);
        if tail == self.cached_head {
            self.cached_head = self.queue.head.load(Ordering::Acquire);
            if tail == self.cached_head {
                return None;
            }
        }

        let idx = (tail % self.len) as usize;

        #[allow(unsafe_code)]
        let value = unsafe{ &mut *self.queue.store[idx].get() };

        Some(SlotView::new(&self.queue.tail, value))
    }

}

impl<T: Send> Cons<T> {

    fn from_queue(queue: Arc<PoolQueue<T>>) -> Self {
        Self {
            len: queue.store.len() as u64,
            queue,
            cached_head: 0,
        }
    }

}


// // SlotView // //

pub struct SlotView<'a, T: Send> {
    counter: &'a AtomicU64,
    value: &'a mut T,
    done: bool,
}

impl<'a, T: Send> SlotView<'a, T> {

    pub fn dismiss(&mut self) {
        self.done = true;
    }

    pub fn commit(mut self) {
        self.done = false;
        std::mem::drop(self)
    }

}

impl<'a, T: Send> SlotView<'a, T> {
    fn new(counter: &'a AtomicU64, value: &'a mut T) -> Self {
        Self {
            counter,
            value,
            done: false
        }
    }
}

impl<'a, T: Send> Drop for SlotView<'a, T> {
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