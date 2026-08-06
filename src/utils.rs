//! Internal helpers shared by dictionary parsers.

use std::fs;
use std::ops::Range;
use std::path::Path;

use crate::{Error, Format, Result};

/// Reads a dictionary file while preserving its format and path in I/O errors.
pub(crate) fn read_file(path: impl AsRef<Path>, format: Format) -> Result<Vec<u8>> {
    let path = path.as_ref();
    fs::read(path).map_err(|source| Error::Io {
        format,
        path: path.to_owned(),
        source,
    })
}

/// Returns a bounded byte range or a structured end-of-input error.
pub(crate) fn slice_at(data: &[u8], offset: usize, length: usize, format: Format) -> Result<&[u8]> {
    let available = data.len().saturating_sub(offset);
    let end = offset
        .checked_add(length)
        .filter(|&end| end <= data.len())
        .ok_or(Error::UnexpectedEof {
            format,
            offset,
            needed: length,
            available,
        })?;

    Ok(&data[offset..end])
}

/// Reads a little-endian `u32` from a specified byte offset.
pub(crate) fn read_u32_at(data: &[u8], offset: usize, format: Format) -> Result<u32> {
    let bytes = slice_at(data, offset, 4, format)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Reads a little-endian `u64` from a specified byte offset.
pub(crate) fn read_u64_at(data: &[u8], offset: usize, format: Format) -> Result<u64> {
    let bytes = slice_at(data, offset, 8, format)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Decodes a fixed-width, NUL-terminated UTF-16LE field.
pub(crate) fn decode_fixed_utf16(
    data: &[u8],
    range: Range<usize>,
    field: &'static str,
    format: Format,
) -> Result<Option<String>> {
    let offset = range.start;
    let bytes = slice_at(data, offset, range.len(), format)?;
    let used_len = bytes
        .chunks_exact(2)
        .position(|pair| pair == [0, 0])
        .map_or(bytes.len(), |index| index * 2);

    if used_len == 0 {
        return Ok(None);
    }

    decode_utf16(&bytes[..used_len], offset, field, format).map(Some)
}

/// Strictly decodes UTF-16LE without allocating an intermediate `Vec<u16>`.
pub(crate) fn decode_utf16(
    bytes: &[u8],
    offset: usize,
    field: &'static str,
    format: Format,
) -> Result<String> {
    ensure_even(field, offset, bytes.len(), format)?;

    std::char::decode_utf16(
        bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]])),
    )
    .collect::<std::result::Result<String, _>>()
    .map_err(|source| Error::InvalidUtf16 {
        format,
        field,
        offset,
        source,
    })
}

/// Ensures that a UTF-16LE field contains complete two-byte code units.
#[allow(clippy::manual_is_multiple_of)]
pub(crate) fn ensure_even(
    field: &'static str,
    offset: usize,
    length: usize,
    format: Format,
) -> Result<()> {
    if length % 2 == 0 {
        Ok(())
    } else {
        Err(Error::OddUtf16ByteLength {
            format,
            field,
            offset,
            length,
        })
    }
}

/// Rejects a declared record count that cannot fit in the available data.
pub(crate) fn validate_count(
    field: &'static str,
    count: u64,
    maximum: usize,
    format: Format,
) -> Result<()> {
    if count <= maximum as u64 {
        Ok(())
    } else {
        Err(Error::InvalidCount {
            format,
            field,
            count,
            maximum: maximum as u64,
        })
    }
}

/// A bounded binary reader that reports source-absolute error offsets.
pub(crate) struct Reader<'data> {
    data: &'data [u8],
    base: usize,
    position: usize,
    format: Format,
}

impl<'data> Reader<'data> {
    /// Creates a reader at an absolute offset in a complete source buffer.
    pub(crate) fn at(data: &'data [u8], position: usize, format: Format) -> Self {
        debug_assert!(position <= data.len());
        Self {
            data,
            base: 0,
            position,
            format,
        }
    }

    /// Creates a reader over a section while retaining its source-file offset.
    pub(crate) fn section(data: &'data [u8], base: usize, format: Format) -> Self {
        Self {
            data,
            base,
            position: 0,
            format,
        }
    }

    pub(crate) fn position(&self) -> usize {
        self.base + self.position
    }

    /// Returns the number of unread bytes in the current buffer or section.
    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    /// Verifies that a bounded section has been consumed exactly.
    pub(crate) fn finish(self, field: &'static str) -> Result<()> {
        if self.position == self.data.len() {
            Ok(())
        } else {
            Err(Error::SizeMismatch {
                format: self.format,
                field,
                expected: self.data.len() as u64,
                actual: self.position as u64,
            })
        }
    }

    /// Reads a little-endian `u16`.
    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a length-delimited UTF-16LE string.
    pub(crate) fn read_utf16(&mut self, length: usize, field: &'static str) -> Result<String> {
        let offset = self.position();
        let bytes = self.read_bytes(length)?;
        decode_utf16(bytes, offset, field, self.format)
    }

    /// Reads a length-delimited ASCII string.
    pub(crate) fn read_ascii(&mut self, length: usize, field: &'static str) -> Result<String> {
        let offset = self.position();
        let bytes = self.read_bytes(length)?;
        let mut text = String::with_capacity(length);

        for (index, &byte) in bytes.iter().enumerate() {
            if !byte.is_ascii() {
                return Err(Error::InvalidAscii {
                    format: self.format,
                    field,
                    offset: offset + index,
                    byte,
                });
            }
            text.push(char::from(byte));
        }

        Ok(text)
    }

    /// Reads exactly `length` bytes and advances the cursor.
    pub(crate) fn read_bytes(&mut self, length: usize) -> Result<&'data [u8]> {
        let available = self.remaining();
        let end = self
            .position
            .checked_add(length)
            .filter(|&end| end <= self.data.len())
            .ok_or(Error::UnexpectedEof {
                format: self.format,
                offset: self.position(),
                needed: length,
                available,
            })?;
        let bytes = &self.data[self.position..end];
        self.position = end;
        Ok(bytes)
    }
}
