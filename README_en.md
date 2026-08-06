# cidian

`cidian` parses Chinese input-method dictionary files into one small, common Rust data model. It supports
Sogou Cell Dictionary (`.scel`), QQ Pinyin Cell Dictionary (`.qcel`), QQ Pinyin category dictionary
(`.qpyd`), and Baidu desktop (`.bdict`) and mobile (`.bcd`) category dictionaries.

The crate is concerned only with parsing: it does not normalize, sort, deduplicate, or export entries.

## Supported formats

  | Format                            | Extension | Module          |
  | --------------------------------- | --------- | --------------- |
  | Sogou Cell Dictionary             | `.scel`   | `cidian::scel`  |
  | QQ Pinyin Cell Dictionary         | `.qcel`   | `cidian::qcel`  |
  | QQ Pinyin category dictionary     | `.qpyd`   | `cidian::qpyd`  |
  | Baidu desktop category dictionary | `.bdict`  | `cidian::bdict` |
  | Baidu mobile category dictionary  | `.bcd`    | `cidian::bcd`   |

## Quick start

Add the dependency:

```toml
[dependencies]
cidian = "0.1"
```

Parse a dictionary from a file:

```rust
use cidian::scel;

let dictionary = scel::parse_file("dictionary.scel")?;

for entry in dictionary.entries {
    println!("{}\t{}\t{:?}", entry.word, entry.code.join(" "), entry.weight);
}
```

For data already in memory, pass the bytes to `parse` instead:

```rust
let dictionary = cidian::qpyd::parse(&bytes)?;
```

Every format module exposes the same two functions: `parse(&[u8]) -> Result<Dictionary>` and
`parse_file(path) -> Result<Dictionary>`.

## Data model

All parsers return the same [`Dictionary`](https://docs.rs/cidian/latest/cidian/struct.Dictionary.html)
type:

- **`metadata`** --- a [`Metadata`](https://docs.rs/cidian/latest/cidian/struct.Metadata.html) with the
  common fields `name`, `category`, and `description` (each `Option<String>`), plus `extra`, a map of
  source-specific strings that have no common field.
- **`entries`** --- entries in source order. Each
  [`Entry`](https://docs.rs/cidian/latest/cidian/struct.Entry.html) has:
  - `word` --- the word or phrase, exactly as stored by the source.
  - `code` --- the coding components in source order, split the way the format stores them: pinyin
    syllables or Latin codes for SCEL and QCEL, apostrophe-delimited pinyin for QPYD, and pinyin
    syllables or the stored code as a single component for Baidu entries.
  - `weight` --- an optional numeric weight defined by the source. QCEL word extensions must contain
    at least four bytes; the parser reads the value from those first four bytes and rejects shorter
    extensions. Vendors do not publish precise statistical semantics, so treat it as a source-defined
    value rather than a corpus frequency. QPYD entries always have `weight: None`.

## Errors

Every parser returns the unified `cidian::Result<T>` with the structured
[`cidian::Error`](https://docs.rs/cidian/latest/cidian/enum.Error.html) type. Errors retain the
dictionary `Format` and relevant details such as the file path, byte offset, and field name, so you can
handle failures without parsing error messages:

```rust
match cidian::scel::parse(&[]) {
    Err(cidian::Error::UnexpectedEof { format, offset, .. }) => {
        println!("truncated {format} data at {offset:#x}");
    }
    Err(error) => println!("{error}"),
    Ok(_) => {}
}
```

Filesystem errors are reported as `Error::Io` and carry the path.

## Behavior

- Entries stay in source order; text is not normalized.
- Parsing is strict: truncated or corrupted input fails with a structured error instead of being silently
  repaired.
- QCEL uses QQ Pinyin's built-in code table when the source table is empty and ignores optional trailing
  sections such as `DELTBL` after the declared main word groups.
- The crate does not convert, merge, or export dictionaries.

## Acknowledgments

- Thanks to nopdan for the [input-method dictionary format series](https://nopdan.com/series/lexicon/)
  and the [nopdan/rose](https://github.com/nopdan/rose) converter library, which served as references for
  the formats parsed by this crate.
- Thanks to the qinwf/cidian project for providing the motivation for this crate.

## License

MIT License
