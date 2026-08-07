# cidian-rs / 词典

<p align="center">
  <a href="https://github.com/Yousa-Mirage/cidian-rs/stargazers"><img src="https://img.shields.io/github/stars/Yousa-Mirage/cidian-rs?style=social" alt="GitHub Stars"></a>
  <a href="https://github.com/Yousa-Mirage/cidian-rs/blob/main/LICENSE"><img src="https://img.shields.io/github/license/Yousa-Mirage/cidian-rs?style=flat" alt="MIT license"></a>
  <a href="https://github.com/Yousa-Mirage/cidian-rs/releases"><img src="https://img.shields.io/github/v/release/Yousa-Mirage/cidian-rs?style=flat" alt="GitHub release"></a>
  <a href="https://crates.io/crates/cidian-rs"><img src="https://img.shields.io/crates/v/cidian-rs.svg?style=flat" alt="crates.io version"></a>
  <a href="https://docs.rs/cidian-rs"><img src="https://img.shields.io/docsrs/cidian-rs?style=flat" alt="docs.rs documentation"></a>
</p>

<p align="center">
  <a href="https://github.com/Yousa-Mirage/cidian-rs/blob/main/README.md">中文</a> ·
  <a href="https://github.com/Yousa-Mirage/cidian-rs/blob/main/README_en.md">English</a>
</p>

`cidian-rs` 将中文输入法词库文件解析为统一、轻量的 Rust 数据模型，目前支持搜狗（`.scel`）、 QQ
拼音（`.qcel`、`.qpyd`）与百度（`.bdict`、`.bcd`）词库。

`cidian-rs` 只负责解析，不进行规范化、排序、去重或导出。截断或损坏的输入返回结构化错误，而非静默修复。

## 支持的格式

  | 格式             | 扩展名   | 模块            |
  | ---------------- | -------- | --------------- |
  | 搜狗细胞词库     | `.scel`  | `cidian::scel`  |
  | QQ 拼音细胞词库  | `.qcel`  | `cidian::qcel`  |
  | QQ 拼音分类词库  | `.qpyd`  | `cidian::qpyd`  |
  | 百度桌面分类词库 | `.bdict` | `cidian::bdict` |
  | 百度手机分类词库 | `.bcd`   | `cidian::bcd`   |

## 快速开始

添加依赖：

```toml
[dependencies]
cidian = "0.1"
```

从文件解析词库：

```rust
use cidian::scel;

let dictionary = scel::parse_file("dictionary.scel")?;

for entry in dictionary.entries {
    println!("{}\t{}\t{:?}", entry.word, entry.code.join(" "), entry.weight);
}
```

如果数据已在内存中，直接传入字节：

```rust
let dictionary = cidian::qpyd::parse(&bytes)?;
```

每个格式模块都暴露相同的两个函数：`parse(&[u8]) -> Result<Dictionary>` 和
`parse_file(path) -> Result<Dictionary>`。

## 数据模型

所有解析器返回统一的 [`Dictionary`](https://docs.rs/cidian/latest/cidian/struct.Dictionary.html) 类型：

- **`metadata`** --- [`Metadata`](https://docs.rs/cidian/latest/cidian/struct.Metadata.html)
  结构：公共字段 `name`、`category`、`description`（均为 `Option<String>`），以及 `extra`
  映射（存放格式特有的额外元数据）。
- **`entries`** --- 按源文件顺序排列的词条。每个
  [`Entry`](https://docs.rs/cidian/latest/cidian/struct.Entry.html) 包含：
  - `word` --- 词或短语，与源文件存储的完全一致。
  - `code` --- 按源格式拆分的编码组件：SCEL/QCEL 为拼音音节或拉丁编码，QPYD
    为按撇号分隔的拼音，百度词条为拼音音节或整体编码。
  - `weight` --- 源定义的可选数值权重；QPYD 恒为 `None`。

## 错误处理

所有解析器返回统一的 `cidian::Result<T>`，错误类型为结构化的
[`cidian::Error`](https://docs.rs/cidian/latest/cidian/enum.Error.html)。错误携带词典的 `Format`
及相关细节（如文件路径、字节偏移、字段名），因此无需解析错误消息即可区分失败原因：

```rust
match cidian::scel::parse(&[]) {
    Err(cidian::Error::UnexpectedEof { format, offset, .. }) => {
        println!("{format} 数据在第 {offset:#x} 字节处被截断");
    }
    Err(error) => println!("{error}"),
    Ok(_) => {}
}
```

文件读取失败会以 `Error::Io` 返回，并携带路径信息。

## 基准测试

对当前测试集里四类解析器对应的最大真实词库各解析 10 次，结果如下：

  | 格式       | fixture             | 文件大小 (KB) | 词条数  | 最小耗时 | 平均耗时 |
  | ---------- | ------------------- | ------------: | ------: | -------: | -------: |
  | SCEL       | `医学词汇大全.scel` |      3,659.37 |  90,047 | 21.20 ms | 22.20 ms |
  | QCEL       | `成语俗语大全.qcel` |      2,219.51 |  66,418 | 10.10 ms | 10.85 ms |
  | QPYD       | `唐诗.qpyd`         |      2,885.19 | 161,674 | 58.27 ms | 61.13 ms |
  | 百度 BDICT | `诗词精选.bdict`    |      2,752.89 | 100,264 | 20.08 ms | 21.50 ms |

## 致谢

- 感谢 nopdan 大佬的[输入法词库解析系列文章](https://nopdan.com/series/lexicon/)以及
  [nopdan/rose](https://github.com/nopdan/rose)
  蔷薇词库转换库，开发过程中大量参考了大佬的博客文章和代码实现。
- 感谢 qinwf/cidian 项目为本 crate 提供了开发动机。

## 许可证

MIT License
