//! Streams one reader into several sinks (hashers, destination writers).
//!
//! The reader runs on the calling thread; for large files each sink gets its own
//! thread, fed through a small bounded channel, so reading the card, hashing and writing
//! every destination all overlap. This is also what makes fan-out a single card read.
//! Buffers are aligned so the same pool works for cache-bypassing reads.

mod buffers;
mod pump;

#[cfg(windows)]
pub(crate) use buffers::BUF_ALIGN;
pub(crate) use buffers::BufferPool;
pub(crate) use pump::{ChunkSink, PumpError, hash_stream, pump};
