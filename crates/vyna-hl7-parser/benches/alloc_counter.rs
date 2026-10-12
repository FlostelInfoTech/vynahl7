//! Counting allocator for memory profiling benchmarks.
//!
//! Wraps the system allocator and tracks allocation count and total bytes allocated.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct CountingAllocator {
    alloc_count: AtomicU64,
    alloc_bytes: AtomicU64,
}

impl Default for CountingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl CountingAllocator {
    pub const fn new() -> Self {
        Self {
            alloc_count: AtomicU64::new(0),
            alloc_bytes: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.alloc_count.store(0, Ordering::SeqCst);
        self.alloc_bytes.store(0, Ordering::SeqCst);
    }

    pub fn alloc_count(&self) -> u64 {
        self.alloc_count.load(Ordering::SeqCst)
    }

    pub fn alloc_bytes(&self) -> u64 {
        self.alloc_bytes.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
        self.alloc_bytes
            .fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
