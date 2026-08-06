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
    /// For SCEL files these components are pinyin syllables or embedded Latin
    /// code letters. For QPYD files they are the apostrophe-delimited pinyin
    /// components stored with each entry. Other dictionary formats may use
    /// another coding system in the same field.
    pub code: Vec<String>,
    /// An optional source-defined numeric weight.
    ///
    /// For SCEL files this is the first little-endian `u16` in the word
    /// extension. Sogou does not document its precise semantics, so this field
    /// intentionally does not promise that the value is a corpus frequency.
    pub weight: Option<u32>,
}
