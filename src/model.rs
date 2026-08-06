use std::collections::BTreeMap;

/// A parsed dictionary and its source metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dictionary {
    /// Metadata supplied by the source dictionary.
    pub metadata: Metadata,
    /// Entries in source order.
    pub entries: Vec<Entry>,
}

/// Metadata shared by supported dictionary formats.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// Human-readable dictionary name.
    pub name: Option<String>,
    /// Dictionary category, when supplied by the source.
    pub category: Option<String>,
    /// Dictionary description, when supplied by the source.
    pub description: Option<String>,
    /// Source-specific textual metadata that has no common field.
    pub extra: BTreeMap<String, String>,
}

/// A single dictionary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The word or phrase exactly as represented by the source.
    pub word: String,
    /// Source coding components in source order.
    ///
    /// The representation depends on the dictionary format:
    ///
    /// - **SCEL**: pinyin syllables, including embedded Latin code letters when
    ///   the source uses them.
    /// - **QCEL**: pinyin syllables, explicit Latin codes, or codes resolved
    ///   through QCEL's built-in table when the source omits its table.
    /// - **QPYD**: apostrophe-delimited pinyin components stored with the entry.
    /// - **BDICT/BCD regular entries**: decoded pinyin syllables or embedded
    ///   Latin code letters.
    /// - **BDICT/BCD English and mixed entries**: the directly stored code as a
    ///   single component.
    /// - **Other formats**: another source-specific coding system may be used.
    pub code: Vec<String>,
    /// An optional source-defined numeric weight.
    ///
    /// The representation depends on the dictionary format:
    ///
    /// - **SCEL**: the first little-endian `u16` in the word extension, when
    ///   the extension contains at least two bytes.
    /// - **QCEL**: the first little-endian `u32` in the word extension. QCEL
    ///   records with an extension shorter than four bytes are malformed.
    /// - **BDICT/BCD**: the numeric field in regular, English, and weighted
    ///   mixed records. Mixed records without a documented weight use `None`.
    /// - **QPYD**: always `None`; its undocumented four-byte index field is not
    ///   exposed as a weight.
    ///
    /// These formats do not document precise statistical semantics, so this
    /// field does not promise a corpus frequency.
    pub weight: Option<u32>,
}
