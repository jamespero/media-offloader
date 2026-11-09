//! Aligned buffers, and the pools that reuse them.

use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::ptr::NonNull;
use std::sync::{Arc, Condvar, Mutex};

/// Read/write size. Large enough for full speed on card readers and SSDs.
pub(crate) const CHUNK_SIZE: usize = 8 << 20;

/// Buffer alignment. Covers 512/4096-byte sectors (Windows unbuffered I/O) and
/// 16 KiB pages (direct I/O on Apple silicon).
pub(crate) const BUF_ALIGN: usize = 64 << 10;

/// Buffers per pool: one being filled, one being consumed, two queued.
const POOL_MAX: usize = 4;

/// A heap buffer with [`BUF_ALIGN`] alignment.
pub(crate) struct AlignedBuf {
    ptr: NonNull<u8>,
    cap: usize,
    pub(super) len: usize,
}

// SAFETY: AlignedBuf uniquely owns its allocation, like a Box<[u8]>.
unsafe impl Send for AlignedBuf {}
unsafe impl Sync for AlignedBuf {}

impl AlignedBuf {
    fn layout(cap: usize) -> Layout {
        Layout::from_size_align(cap, BUF_ALIGN).expect("valid buffer layout")
    }

    fn new(cap: usize) -> Self {
        let layout = Self::layout(cap);
        // SAFETY: layout has non-zero size. Zeroed so handing out &mut [u8] is sound.
        let raw = unsafe { alloc_zeroed(layout) };
        let ptr = NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(layout));
        Self { ptr, cap, len: 0 }
    }

    pub fn data(&self) -> &[u8] {
        // SAFETY: len <= cap and the memory is initialized.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub(super) fn whole_mut(&mut self) -> &mut [u8] {
        // SAFETY: we own cap initialized bytes.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.cap) }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        // SAFETY: allocated in `new` with the same layout.
        unsafe { dealloc(self.ptr.as_ptr(), Self::layout(self.cap)) }
    }
}

/// Recycles a few large aligned buffers. Use one pool per concurrent pump: a pump
/// blocks for a free buffer, so sharing a pool between concurrent pumps could stall.
#[derive(Clone)]
pub(crate) struct BufferPool(Arc<PoolInner>);

struct PoolInner {
    state: Mutex<(Vec<AlignedBuf>, usize)>, // (free buffers, total allocated)
    returned: Condvar,
}

impl BufferPool {
    pub fn new() -> Self {
        Self(Arc::new(PoolInner { state: Mutex::new((Vec::new(), 0)), returned: Condvar::new() }))
    }

    pub(super) fn take(&self) -> AlignedBuf {
        let mut st = self.0.state.lock().unwrap();
        loop {
            if let Some(mut b) = st.0.pop() {
                b.len = 0;
                return b;
            }
            if st.1 < POOL_MAX {
                st.1 += 1;
                return AlignedBuf::new(CHUNK_SIZE);
            }
            st = self.0.returned.wait(st).unwrap();
        }
    }

    pub(super) fn give(&self, buf: AlignedBuf) {
        self.0.state.lock().unwrap().0.push(buf);
        self.0.returned.notify_one();
    }
}

/// A filled buffer shared by every sink; goes back to the pool when the last one is done.
pub(super) struct Chunk {
    pub(super) buf: Option<AlignedBuf>,
    pub(super) pool: BufferPool,
}

impl Chunk {
    pub(super) fn data(&self) -> &[u8] {
        self.buf.as_ref().map_or(&[], AlignedBuf::data)
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        if let Some(b) = self.buf.take() {
            self.pool.give(b);
        }
    }
}
