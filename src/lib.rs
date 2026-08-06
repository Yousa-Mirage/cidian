//! Parsers for Chinese input-method dictionary formats.
//!
//! `cidian` converts source-specific dictionary files into a small common data
//! model. It deliberately does not normalize entries or export them to another
//! dictionary format.

#![deny(warnings)]
#![deny(dead_code)]
#![deny(missing_docs)]

mod baidu;
mod error;
mod model;

pub mod qcel;
pub mod qpyd;
pub mod scel;

pub use baidu::{bcd, bdict};
pub use error::{Error, Format, Result};
pub use model::{Dictionary, Entry, Metadata};
