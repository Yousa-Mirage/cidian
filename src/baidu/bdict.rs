//! Parser for Baidu desktop category dictionary (`.bdict`) files.

use std::path::Path;

use super::{BaiduVariant, parse as parse_baidu, parse_file as parse_baidu_file};
use crate::{Dictionary, Result};

/// Parses a BDICT dictionary from memory.
///
/// Entries are returned in the format's regular, English, then mixed section
/// order. Text and coding strings are not normalized.
///
/// # Examples
///
/// ```no_run
/// let bytes = std::fs::read("dictionary.bdict")?;
/// let dictionary = cidian::bdict::parse(&bytes)?;
/// println!("{} entries", dictionary.entries.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse(data: &[u8]) -> Result<Dictionary> {
    parse_baidu(data, BaiduVariant::Bdict)
}

/// Parses a BDICT dictionary from a file.
///
/// The result is identical to reading the file and passing its bytes to
/// [`parse`]. The file name does not alter dictionary metadata.
///
/// # Examples
///
/// ```no_run
/// let dictionary = cidian::bdict::parse_file("dictionary.bdict")?;
/// println!("{:?}", dictionary.metadata.name);
/// # Ok::<(), cidian::Error>(())
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Dictionary> {
    parse_baidu_file(path, BaiduVariant::Bdict)
}
