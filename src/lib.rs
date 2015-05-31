#![feature(core, rustc_private, collections)]

extern crate arena;

pub mod iter;
pub mod str;
pub mod slice;
pub mod exts;

pub use iter::{IterTools, StreamingIterator};
pub use str::StringTools;
pub use slice::{SliceTools, VecTools};
