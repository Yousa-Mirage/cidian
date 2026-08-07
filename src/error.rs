use std::char::DecodeUtf16Error;
use std::collections::TryReserveError;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::str::Utf8Error;

/// A dictionary format understood by `cidian-rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Baidu mobile category dictionary (`.bcd`).
    Bcd,
    /// Baidu desktop category dictionary (`.bdict`).
    Bdict,
    /// QQ Pinyin category dictionary (`.qpyd`).
    Qpyd,
    /// QQ Pinyin Cell Dictionary (`.qcel`).
    Qcel,
    /// Sogou Cell Dictionary (`.scel`).
    Scel,
}

impl fmt::Display for Format {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bcd => formatter.write_str("BCD"),
            Self::Bdict => formatter.write_str("BDICT"),
            Self::Qpyd => formatter.write_str("QPYD"),
            Self::Qcel => formatter.write_str("QCEL"),
            Self::Scel => formatter.write_str("SCEL"),
        }
    }
}

/// Errors returned while reading or parsing a dictionary.
///
/// Every variant retains the source format and structured diagnostic data so
/// callers can distinguish malformed input, unsupported data, and I/O errors
/// without inspecting the displayed error message.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A dictionary file could not be read.
    #[error("failed to read {format} dictionary `{path}`: {source}")]
    Io {
        /// Dictionary format requested by the caller.
        format: Format,
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },

    /// A read extended past the available input.
    #[error(
        "unexpected end of {format} data at byte {offset:#x}: needed {needed} bytes, {available} available"
    )]
    UnexpectedEof {
        /// Dictionary format being parsed.
        format: Format,
        /// Byte offset at which the read was attempted.
        offset: usize,
        /// Number of bytes required by the read.
        needed: usize,
        /// Number of bytes available from `offset`.
        available: usize,
    },

    /// The dictionary magic bytes were invalid.
    #[error("invalid {format} magic: found {found:02x?}")]
    InvalidMagic {
        /// Dictionary format being parsed.
        format: Format,
        /// Magic bytes found in the input.
        found: Vec<u8>,
    },

    /// The container variant was not supported.
    #[error("unsupported {format} variant: found {found:02x?}")]
    UnsupportedVariant {
        /// Dictionary format being parsed.
        format: Format,
        /// Variant bytes found in the input.
        found: Vec<u8>,
    },

    /// The format version was not supported.
    #[error("unsupported {format} version: found {found:02x?}")]
    UnsupportedVersion {
        /// Dictionary format being parsed.
        format: Format,
        /// Version bytes found in the input.
        found: Vec<u8>,
    },

    /// A UTF-16LE field declared an odd number of bytes.
    #[error("{field} in {format} data at byte {offset:#x} has odd UTF-16LE byte length {length}")]
    OddUtf16ByteLength {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the field being parsed.
        field: &'static str,
        /// Offset of the field contents.
        offset: usize,
        /// Declared byte length.
        length: usize,
    },

    /// A field contained invalid UTF-16.
    #[error("invalid UTF-16LE in {format} field `{field}` at byte {offset:#x}: {source}")]
    InvalidUtf16 {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the field being parsed.
        field: &'static str,
        /// Offset of the field contents.
        offset: usize,
        /// Underlying UTF-16 decoder error.
        #[source]
        source: DecodeUtf16Error,
    },

    /// A field contained invalid UTF-8.
    #[error("invalid UTF-8 in {format} field `{field}` at byte {offset:#x}: {source}")]
    InvalidUtf8 {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the field being parsed.
        field: &'static str,
        /// Offset of the field contents.
        offset: usize,
        /// Underlying UTF-8 decoder error.
        #[source]
        source: Utf8Error,
    },

    /// A field declared as ASCII contained a non-ASCII byte.
    #[error("invalid ASCII byte {byte:#04x} in {format} field `{field}` at byte {offset:#x}")]
    InvalidAscii {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the field being parsed.
        field: &'static str,
        /// Byte offset of the invalid value.
        offset: usize,
        /// Invalid byte value.
        byte: u8,
    },

    /// A compressed data section could not be decoded.
    #[error(
        "failed to decompress {format} data at byte {offset:#x}: zlib returned status {status}"
    )]
    InvalidCompression {
        /// Dictionary format being parsed.
        format: Format,
        /// Byte offset of the compressed section in the source file.
        offset: usize,
        /// Numeric zlib return status.
        status: i32,
    },

    /// A declared byte offset was outside its valid section.
    #[error(
        "invalid {field} offset {offset:#x} in {format} data: expected {minimum:#x}..={maximum:#x}"
    )]
    InvalidOffset {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the offset field.
        field: &'static str,
        /// Declared byte offset.
        offset: usize,
        /// Smallest valid byte offset.
        minimum: usize,
        /// Largest valid byte offset.
        maximum: usize,
    },

    /// A decoded section size disagreed with the source declaration.
    #[error("{field} size mismatch in {format} data: expected {expected}, decoded {actual}")]
    SizeMismatch {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the measured section.
        field: &'static str,
        /// Size declared by the source.
        expected: u64,
        /// Size produced while parsing.
        actual: u64,
    },

    /// Memory for a declared section could not be reserved.
    #[error("failed to reserve {requested} items for {field} while parsing {format}: {source}")]
    AllocationFailed {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the allocation target.
        field: &'static str,
        /// Number of items requested.
        requested: usize,
        /// Underlying allocation error.
        #[source]
        source: TryReserveError,
    },

    /// A declared count cannot fit in the remaining input.
    #[error(
        "invalid {field} count {count} in {format} data: at most {maximum} records fit in the remaining data"
    )]
    InvalidCount {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the count field.
        field: &'static str,
        /// Declared count.
        count: u64,
        /// Maximum count possible from the remaining bytes.
        maximum: u64,
    },

    /// A code table contains the same identifier more than once.
    #[error("duplicate code index {index} in {format} data")]
    DuplicateCodeIndex {
        /// Dictionary format being parsed.
        format: Format,
        /// Duplicate code identifier.
        index: u64,
    },

    /// A dictionary entry refers to an identifier absent from its code table.
    #[error("undefined code index {index} in {format} data at byte {offset:#x}")]
    InvalidCodeIndex {
        /// Dictionary format being parsed.
        format: Format,
        /// Unknown code identifier.
        index: u64,
        /// Byte offset of the reference.
        offset: usize,
    },

    /// A component of an encoded syllable was outside its lookup table.
    #[error("invalid {field} index {index} in {format} data at byte {offset:#x}")]
    InvalidCodeComponent {
        /// Dictionary format being parsed.
        format: Format,
        /// Component lookup table, such as `initial` or `final`.
        field: &'static str,
        /// Invalid lookup-table index.
        index: u64,
        /// Byte offset of the invalid index.
        offset: usize,
    },

    /// A record header did not match any supported layout.
    #[error("invalid {format} {field} record header at byte {offset:#x}: found {found:02x?}")]
    InvalidRecordHeader {
        /// Dictionary format being parsed.
        format: Format,
        /// Kind of record whose header was invalid.
        field: &'static str,
        /// Byte offset of the record header.
        offset: usize,
        /// Header bytes found in the input.
        found: Vec<u8>,
    },

    /// A record field is shorter than the format requires.
    #[error(
        "invalid {field} length {length} in {format} data at byte {offset:#x}: at least {minimum} bytes are required"
    )]
    InvalidExtensionLength {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the extension field.
        field: &'static str,
        /// Byte offset of the length field.
        offset: usize,
        /// Declared extension length.
        length: usize,
        /// Minimum length required by the format.
        minimum: usize,
    },

    /// A parsed item count disagrees with its declared count.
    #[error("{field} count mismatch in {format} data: expected {expected}, parsed {actual}")]
    CountMismatch {
        /// Dictionary format being parsed.
        format: Format,
        /// Name of the counted item.
        field: &'static str,
        /// Count declared by the source.
        expected: u64,
        /// Number of items actually parsed.
        actual: u64,
    },
}

/// Result type used by all dictionary parsers.
pub type Result<T> = std::result::Result<T, Error>;
