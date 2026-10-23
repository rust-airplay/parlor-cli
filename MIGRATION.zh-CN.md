# 迁移：程序独立成仓库

English: [MIGRATION.md](MIGRATION.md)。

这个 crate 原本是库工作区里的 `crates/rust-airplay-cli`，产出的可执行文件叫 `rust-airplay`。现在它是 `parlor-cli` 仓库，产出 `parlor`。

## 变化

| 之前 | 现在 |
| --- | --- |
| 包名 `rust-airplay-cli` | 包名 `parlor-cli` |
| 可执行文件 `rust-airplay` | 可执行文件 `parlor` |
| 默认接收端名 `rust-airplay` | 默认接收端名 `parlor` |
| 数据目录 `rust-airplay/` | 数据目录 `parlor/` |
| 路径依赖 | 对两个库的 Git 依赖 |
| `rust_airplay_gstreamer::` | `rust_airplay_gstream::` |

`src/main.rs` 其余部分未变：仍然解析参数、保存身份种子与客户端列表、把库的 `log` 记录输出到终端。

数据目录改了名字，因此已有的配对不会自动沿用。要保留身份，把目录移过去即可：

```powershell
Move-Item "$env:LOCALAPPDATA\rust-airplay" "$env:LOCALAPPDATA\parlor"
```

## 依赖

```toml
rust-airplay = { git = "https://github.com/rust-airplay/rust-airplay", branch = "main" }
rust-airplay-gstream = { git = "https://github.com/rust-airplay/rust-airplay-gstream", branch = "main", optional = true }
```

两者都是 Git 依赖，所以这个仓库要在两个库推送之后才能构建。`gst` 特性默认关闭，`--render null` 既不需要 GStreamer，也不需要后端。

## 门禁

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features gst -- -D warnings
cargo build --release --features gst
```

这里没有单测：这个 crate 拥有的行为是参数解析与那两个文件，会话本身由库的测试覆盖。
