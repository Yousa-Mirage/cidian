# cidian

`cidian` parses Chinese input-method dictionary files into a small, common
Rust data model. Version 0.1 supports Sogou Cell Dictionary (`.scel`) and QQ
Pinyin category dictionary (`.qpyd`) files.

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

For data already in memory, use `cidian::scel::parse(&bytes)`. QPYD files use
the corresponding `cidian::qpyd::parse()` and `cidian::qpyd::parse_file()`
functions.

## Data model

Every parser returns a [`Dictionary`](https://docs.rs/cidian/latest/cidian/struct.Dictionary.html)
containing source metadata and a list of entries. An entry contains its word,
structured coding components, and an optional source-defined numeric weight.
For SCEL files, the coding components are pinyin syllables or embedded Latin
code letters. For QPYD files, the apostrophe-delimited pinyin stored with each
entry is returned as separate coding components.

For SCEL files, the weight comes from the first little-endian 16-bit value in
the word extension. Sogou does not publish the exact meaning of this value, so
`cidian` does not describe it as a frequency.

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
- retains the header version, raw FILETIME (`filetime_raw`), first-level category, examples, and
  unknown labelled metadata in `Metadata::extra`.

QPYD's four-byte index field is undocumented and is not exposed as a weight.
Consequently, QPYD entries have `weight: None`.

## Development and tests

The unit tests construct small SCEL and QPYD byte sequences in memory to
exercise individual parser branches. The integration tests additionally parse
real samples from `tests/fixtures/<format>/` and check golden properties such
as entry counts, metadata, boundary entries, and representative entries. Test
fixtures are excluded from published Cargo packages.

Run the full local test suite with:

```text
just test
```

## License

MIT License
