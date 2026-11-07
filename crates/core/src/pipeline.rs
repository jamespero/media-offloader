//! Streams one reader into several sinks (hashers, destination writers).
//!
//! The reader runs on the calling thread; for large files each sink gets its own
//! thread, fed through a small bounded channel, so reading the card, hashing and writing
//! every destination all overlap. This is also what makes fan-out a single card read.
//! Buffers are aligned so the same pool works for cache-bypassing reads.

use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::io::{self, Read};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Condvar, Mutex};

use crate::hash::{HashConfig, Hashes, Md5Sink, Xxh64Sink};

/// Read/write size. Large enough for full speed on card readers and SSDs.
pub(crate) const CHUNK_SIZE: usize = 8 << 20;

/// Buffer alignment. Covers 512/4096-byte sectors (Windows unbuffered I/O) and
/// 16 KiB pages (direct I/O on Apple silicon).
pub(crate) const BUF_ALIGN: usize = 64 << 10;

/// Buffers per pool: one being filled, one being consumed, two queued.
const POOL_MAX: usize = 4;
const QUEUE_DEPTH: usize = 2;

/// Something that consumes a stream of chunks.
pub(crate) trait ChunkSink: Send {
    fn consume(&mut self, data: &[u8]) -> io::Result<()>;
}

pub(crate) enum PumpError {
    Cancelled,
    Read(io::Error),
}

pub(crate) struct Pumped {
    pub bytes: u64,
    /// One entry per sink, in order: the first error that sink hit, if any.
    #[allow(dead_code)] // read by the copy job
    pub sink_errors: Vec<Option<io::Error>>,
}

/// A heap buffer with [`BUF_ALIGN`] alignment.
pub(crate) struct AlignedBuf {
    ptr: NonNull<u8>,
    cap: usize,
    len: usize,
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

    fn whole_mut(&mut self) -> &mut [u8] {
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

    fn take(&self) -> AlignedBuf {
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

    fn give(&self, buf: AlignedBuf) {
        self.0.state.lock().unwrap().0.push(buf);
        self.0.returned.notify_one();
    }
}

/// A filled buffer shared by every sink; goes back to the pool when the last one is done.
struct Chunk {
    buf: Option<AlignedBuf>,
    pool: BufferPool,
}

impl Chunk {
    fn data(&self) -> &[u8] {
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

/// Reads until `buf` is full or the reader is exhausted.
fn fill(reader: &mut dyn Read, buf: &mut AlignedBuf) -> io::Result<usize> {
    let space = buf.whole_mut();
    let mut n = 0;
    while n < space.len() {
        match reader.read(&mut space[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    buf.len = n;
    Ok(n)
}

/// Feeds everything from `reader` to every sink. A sink that errors stops receiving
/// data but the others carry on; a read error or cancellation stops everything.
/// `progress` gets the running byte count after each chunk.
pub(crate) fn pump(
    reader: &mut dyn Read,
    sinks: &mut [&mut dyn ChunkSink],
    expected_len: u64,
    pool: &BufferPool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<Pumped, PumpError> {
    // Small files: threads would cost more than they save.
    if expected_len <= CHUNK_SIZE as u64 || sinks.len() == 1 && expected_len <= 2 * CHUNK_SIZE as u64 {
        return pump_inline(reader, sinks, pool, cancel, progress);
    }

    std::thread::scope(|scope| {
        let mut senders = Vec::with_capacity(sinks.len());
        let mut workers = Vec::with_capacity(sinks.len());
        for sink in sinks.iter_mut() {
            let (tx, rx) = sync_channel::<Arc<Chunk>>(QUEUE_DEPTH);
            senders.push(Some(tx));
            workers.push(scope.spawn(move || {
                for chunk in rx {
                    if let Err(e) = sink.consume(chunk.data()) {
                        return Some(e); // dropping rx tells the reader to stop sending
                    }
                }
                None
            }));
        }

        let mut total = 0u64;
        let mut result = Ok(());
        loop {
            if cancel.load(Ordering::Relaxed) {
                result = Err(PumpError::Cancelled);
                break;
            }
            if senders.iter().all(Option::is_none) {
                break; // every sink failed; nothing left to feed
            }
            let mut buf = pool.take();
            let n = match fill(reader, &mut buf) {
                Ok(n) => n,
                Err(e) => {
                    pool.give(buf);
                    result = Err(PumpError::Read(e));
                    break;
                }
            };
            if n == 0 {
                pool.give(buf);
                break;
            }
            total += n as u64;
            let chunk = Arc::new(Chunk { buf: Some(buf), pool: pool.clone() });
            for slot in senders.iter_mut() {
                if let Some(tx) = slot {
                    if tx.send(Arc::clone(&chunk)).is_err() {
                        *slot = None;
                    }
                }
            }
            drop(chunk);
            progress(total);
        }
        drop(senders);
        let sink_errors = workers.into_iter().map(|w| w.join().expect("sink thread panicked")).collect();
        result.map(|()| Pumped { bytes: total, sink_errors })
    })
}

fn pump_inline(
    reader: &mut dyn Read,
    sinks: &mut [&mut dyn ChunkSink],
    pool: &BufferPool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<Pumped, PumpError> {
    let mut sink_errors: Vec<Option<io::Error>> = sinks.iter().map(|_| None).collect();
    let mut buf = pool.take();
    let mut total = 0u64;
    let result = loop {
        if cancel.load(Ordering::Relaxed) {
            break Err(PumpError::Cancelled);
        }
        let n = match fill(reader, &mut buf) {
            Ok(0) => break Ok(()),
            Ok(n) => n,
            Err(e) => break Err(PumpError::Read(e)),
        };
        total += n as u64;
        for (sink, err) in sinks.iter_mut().zip(sink_errors.iter_mut()) {
            if err.is_none() {
                if let Err(e) = sink.consume(buf.data()) {
                    *err = Some(e);
                }
            }
        }
        progress(total);
    };
    pool.give(buf);
    result.map(|()| Pumped { bytes: total, sink_errors })
}

/// Hashes everything from `reader`. Returns the hashes and the byte count.
pub(crate) fn hash_stream(
    reader: &mut dyn Read,
    expected_len: u64,
    cfg: HashConfig,
    pool: &BufferPool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<(Hashes, u64), PumpError> {
    let mut xxh = Xxh64Sink::new();
    let mut md5 = cfg.md5.then(Md5Sink::new);
    let pumped = {
        let mut sinks: Vec<&mut dyn ChunkSink> = vec![&mut xxh];
        if let Some(m) = md5.as_mut() {
            sinks.push(m);
        }
        pump(reader, &mut sinks, expected_len, pool, cancel, progress)?
    };
    let hashes = Hashes { xxh64: xxh.finish(), md5: md5.map(Md5Sink::finish) };
    Ok((hashes, pumped.bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Hasher;

    struct Collect(Vec<u8>);
    impl ChunkSink for Collect {
        fn consume(&mut self, d: &[u8]) -> io::Result<()> {
            self.0.extend_from_slice(d);
            Ok(())
        }
    }
    struct FailAfter(usize);
    impl ChunkSink for FailAfter {
        fn consume(&mut self, _: &[u8]) -> io::Result<()> {
            if self.0 == 0 {
                return Err(io::Error::other("disk gone"));
            }
            self.0 -= 1;
            Ok(())
        }
    }

    fn data(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect()
    }

    #[test]
    fn threaded_pump_delivers_everything_to_every_sink() {
        let src = data(3 * CHUNK_SIZE + 12345);
        let (mut a, mut b) = (Collect(Vec::new()), Collect(Vec::new()));
        let pool = BufferPool::new();
        let cancel = AtomicBool::new(false);
        let mut last = 0;
        let p = pump(&mut &src[..], &mut [&mut a, &mut b], src.len() as u64, &pool, &cancel, &mut |n| last = n)
            .ok()
            .unwrap();
        assert_eq!(p.bytes, src.len() as u64);
        assert_eq!(last, src.len() as u64);
        assert!(a.0 == src && b.0 == src);
    }

    #[test]
    fn failing_sink_does_not_stop_the_others() {
        let src = data(4 * CHUNK_SIZE);
        let mut good = Collect(Vec::new());
        let mut bad = FailAfter(1);
        let pool = BufferPool::new();
        let cancel = AtomicBool::new(false);
        let p = pump(&mut &src[..], &mut [&mut bad, &mut good], src.len() as u64, &pool, &cancel, &mut |_| {})
            .ok()
            .unwrap();
        assert!(p.sink_errors[0].is_some() && p.sink_errors[1].is_none());
        assert!(good.0 == src);
    }

    #[test]
    fn hash_stream_matches_one_shot_hash() {
        for len in [0, 1, CHUNK_SIZE - 1, CHUNK_SIZE, CHUNK_SIZE + 1, 3 * CHUNK_SIZE + 7] {
            let src = data(len);
            let cfg = HashConfig { md5: true };
            let mut h = Hasher::new(cfg);
            h.update(&src);
            let pool = BufferPool::new();
            let (got, n) =
                hash_stream(&mut &src[..], len as u64, cfg, &pool, &AtomicBool::new(false), &mut |_| {}).ok().unwrap();
            assert_eq!(n, len as u64);
            assert_eq!(got, h.finish(), "len {len}");
        }
    }

    #[test]
    fn cancel_stops_the_pump() {
        let src = data(3 * CHUNK_SIZE);
        let mut a = Collect(Vec::new());
        let pool = BufferPool::new();
        let cancel = AtomicBool::new(true);
        let r = pump(&mut &src[..], &mut [&mut a], src.len() as u64, &pool, &cancel, &mut |_| {});
        assert!(matches!(r, Err(PumpError::Cancelled)));
    }
}
