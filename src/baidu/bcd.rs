//! Parser for Baidu mobile category dictionary (`.bcd`) files.

use std::path::Path;

use super::{BaiduVariant, parse as parse_baidu, parse_file as parse_baidu_file};
use crate::{Dictionary, Result};

/// Parses a BCD dictionary from memory.
///
/// BCD shares Baidu's category-dictionary entry encoding with BDICT while
/// using a mobile header layout. Text and coding strings are not normalized.
///
/// # Examples
///
/// ```no_run
/// let bytes = std::fs::read("dictionary.bcd")?;
/// let dictionary = cidian::bcd::parse(&bytes)?;
/// println!("{} entries", dictionary.entries.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse(data: &[u8]) -> Result<Dictionary> {
    parse_baidu(data, BaiduVariant::Bcd)
}

/// Parses a BCD dictionary from a file.
///
/// The result is identical to reading the file and passing its bytes to
/// [`parse`]. The file name does not alter dictionary metadata.
///
/// # Examples
///
/// ```no_run
/// let dictionary = cidian::bcd::parse_file("dictionary.bcd")?;
/// println!("{:?}", dictionary.metadata.name);
/// # Ok::<(), cidian::Error>(())
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Dictionary> {
    parse_baidu_file(path, BaiduVariant::Bcd)
}
