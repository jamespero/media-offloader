//! Streaming one reader into several sinks.

use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;

use super::buffers::{AlignedBuf, BufferPool, CHUNK_SIZE, Chunk};
use crate::hash::{HashConfig, Hashes, Md5Sink, Xxh64Sink};

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
    pub sink_errors: Vec<Option<io::Error>>,
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
                if let Some(tx) = slot
                    && tx.send(Arc::clone(&chunk)).is_err()
                {
                    *slot = None;
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
            if err.is_none()
                && let Err(e) = sink.consume(buf.data())
            {
                *err = Some(e);
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
