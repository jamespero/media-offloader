//! ASC MHL v2 manifests (`ascmhl/` folders) and classic `.md5` lists: written next to
//! the footage so any copy can be re-verified later, by this app or any MHL tool.
//!
//! Follows the ASC reference implementation (github.com/ascmitc/mhl): generations are
//! `ascmhl/NNNN_<folder>_<YYYY-MM-DD_HHMMSS>Z.mhl`, listed in `ascmhl/ascmhl_chain.xml`
//! with the C4 ID of each manifest file. Directory and root hashes are optional in the
//! schema and not written.

mod hashing;
mod ignore;
mod md5_list;
mod model;
mod reader;
mod writer;

pub use hashing::{HashFormat, MultiHasher, c4_id, c4_of_file};
pub use ignore::ignored;
pub(crate) use md5_list::write_md5_list;
pub use md5_list::{MD5_LIST, read_md5_list};
pub use model::{Entry, Expected, Generation, HashValue, History};
pub use writer::hostname;
pub(crate) use writer::{NewEntry, write_generation};

pub const FOLDER: &str = "ascmhl";
pub const CHAIN_FILE: &str = "ascmhl_chain.xml";
