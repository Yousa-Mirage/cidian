# cidian

`cidian` parses Chinese input-method dictionary files into a small, common
Rust data model. Version 0.1 supports Sogou Cell Dictionary (`.scel`), QQ
Pinyin category dictionary (`.qpyd`), Baidu desktop category dictionary
(`.bdict`), and Baidu mobile category dictionary (`.bcd`) files.

The crate is intentionally concerned only with parsing. It does not normalize,
sort, deduplicate, or export dictionary entries, and it does not provide
bindings for other languages.

## Usage

```rust
let dictionary = cidian::scel::parse_file("dictionary.scel")?;

println!("{:?}", dictionary.metadata.name);
for entry in dictionary.entries {
    println!(
        "{}\t{}\t{:?}",
        entry.word,
        entry.code.join(" "),
        entry.weight
    );
}

# Ok::<(), cidian::Error>(())
```

For data already in memory, use `cidian::scel::parse(&bytes)`. The other
formats expose the same pair of functions under `cidian::qpyd`,
`cidian::bdict`, and `cidian::bcd`.

## Data model

Every parser returns a [`Dictionary`](https://docs.rs/cidian/latest/cidian/struct.Dictionary.html)
containing source metadata and a list of entries. An entry contains its word,
structured coding components, and an optional source-defined numeric weight.
For SCEL files, the coding components are pinyin syllables or embedded Latin
code letters. For QPYD files, the apostrophe-delimited pinyin stored with each
entry is returned as separate coding components. Regular Baidu entries expose
decoded pinyin or embedded Latin letters as components; directly stored
English and mixed codes remain one component.

For SCEL files, the weight comes from the first little-endian 16-bit value in
the word extension. Most Baidu record layouts likewise contain a 16-bit numeric
weight. The vendors do not publish precise statistical semantics for these
values, so `cidian` does not describe them as corpus frequencies.

## Errors

Every parser returns `cidian::Result<T>` with the unified, structured
`cidian::Error` type. Errors retain their dictionary `Format` and relevant
details such as the file path, byte offset, field name, code index, and
expected/actual counts. Callers can therefore handle individual failures
without inspecting error messages:

```rust
match cidian::scel::parse(&[]) {
    Err(cidian::Error::UnexpectedEof {
        format: cidian::Format::Scel,
        offset,
        ..
    }) => println!("truncated SCEL data at {offset:#x}"),
    Err(error) => println!("{error}"),
    Ok(_) => {}
}
```

## SCEL parsing behavior

The parser:

- supports DCS and ECS headers;
- follows the declared pinyin, word-group, and total-word counts;
- computes the word-table offset from the variable-length pinyin table;
- resolves pinyin through the identifiers stored in the file and supports the
  implicit English alphabet encoded after the pinyin table;
- reads each word extension using its declared byte length;
- validates bounds, UTF-16LE, pinyin references, and the final word count;
- stops after the declared main word table, leaving optional trailing sections
  uninterpreted.

Entries remain in source order and textual values are not cleaned or otherwise
normalized.

## QPYD parsing behavior

The parser:

- follows the declared metadata and compressed-section offsets and sizes;
- decodes the information section as strict UTF-16LE;
- validates and decompresses the zlib entry section;
- follows the declared entry count and each payload offset;
- decodes entry codes as strict UTF-8 and words as strict UTF-16LE;
- splits apostrophe-delimited pinyin into structured coding components;
- retains the header version, raw FILETIME (`filetime_raw`), first-level
  category, examples, and unknown labelled metadata in `Metadata::extra`.

QPYD's four-byte index field is undocumented and is not exposed as a weight.
Consequently, QPYD entries have `weight: None`.

## Baidu BDICT and BCD parsing behavior

The BDICT and BCD public modules share a parser core while retaining distinct
format identifiers and error context. The parser:

- validates the common `biptbdsw` header and supported version;
- follows BDICT's declared regular, English, and mixed section offsets, sizes,
  and counts;
- supports BCD's mobile layout, whose regular entries begin at `0x350` while
  its section descriptors are zero;
- decodes regular pinyin through the fixed 24-initial and 33-final lookup
  tables, including embedded Latin characters;
- parses ASCII English entries and both mixed-record headers found in real
  dictionaries;
- decodes metadata and Chinese text as strict UTF-16LE;
- validates section bounds, record counts, code indices, and exact section
  consumption;
- retains the dictionary author and example under `Metadata::extra`.

Entries are returned in regular, English, then mixed section order. Source
records are not filtered: a small number of real BDICT mixed records declare an
empty word, and these remain represented as empty `Entry::word` values.

## Development and tests

The unit tests construct small SCEL, QPYD, BDICT, and BCD byte sequences in
memory to exercise individual parser branches. The integration tests
additionally parse real samples from `tests/fixtures/<format>/` and check golden
properties such as entry counts, metadata, boundary entries, and representative
entries. Test fixtures are excluded from published Cargo packages.

Run the full local test suite with:

```text
just test
```

## License

MIT License
