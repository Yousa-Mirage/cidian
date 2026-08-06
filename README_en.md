# cidian

`cidian` parses Chinese input-method dictionary files into a small, common Rust data model. It supports
Sogou (`.scel`), QQ Pinyin (`.qcel`, `.qpyd`), and Baidu (`.bdict`, `.bcd`) dictionaries.

`cidian` is concerned only with parsing: it does not normalize, sort, deduplicate, or export entries.
Truncated or corrupted input fails with a structured error instead of being silently repaired.

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
  source-specific metadata.
- **`entries`** --- entries in source order. Each
  [`Entry`](https://docs.rs/cidian/latest/cidian/struct.Entry.html) has:
  - `word` --- the word or phrase, exactly as stored by the source.
  - `code` --- the coding components in source order: pinyin syllables or Latin codes for SCEL/QCEL,
    apostrophe-delimited pinyin for QPYD, and pinyin syllables or the stored code as a single component
    for Baidu entries.
  - `weight` --- an optional source-defined numeric weight (`None` for QPYD).

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

## Acknowledgments

- Thanks to nopdan for the [input-method dictionary format series](https://nopdan.com/series/lexicon/)
  and the [nopdan/rose](https://github.com/nopdan/rose) converter library, which served as references for
  the formats parsed by this crate.
- Thanks to the qinwf/cidian project for providing the motivation for this crate.

## License

MIT License
