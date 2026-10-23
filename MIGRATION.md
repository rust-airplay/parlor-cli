# Migration: the program becomes its own repository

中文见 [MIGRATION.zh-CN.md](MIGRATION.zh-CN.md)。

This crate was `crates/rust-airplay-cli` inside the library's workspace, producing a binary
called `rust-airplay`. It is the `parlor-cli` repository now, producing `parlor`.

## What changed

| Before | Now |
| --- | --- |
| package `rust-airplay-cli` | package `parlor-cli` |
| binary `rust-airplay` | binary `parlor` |
| default receiver name `rust-airplay` | default receiver name `parlor` |
| data directory `rust-airplay/` | data directory `parlor/` |
| path dependencies | Git dependencies on both libraries |
| `rust_airplay_gstreamer::` | `rust_airplay_gstream::` |

`src/main.rs` is otherwise unchanged: it still parses arguments, keeps the identity seed and
the client list, and installs a logger over the library's `log` records.

The data directory changed name, so an existing pairing does not carry over. If you want to
keep the identity, move the directory:

```powershell
Move-Item "$env:LOCALAPPDATA\rust-airplay" "$env:LOCALAPPDATA\parlor"
```

## Dependencies

```toml
rust-airplay = { git = "https://github.com/rust-airplay/rust-airplay", branch = "main" }
rust-airplay-gstream = { git = "https://github.com/rust-airplay/rust-airplay-gstream", branch = "main", optional = true }
```

Both are Git dependencies, so this repository builds only after the two libraries are on
their remotes. The `gst` feature is off by default, and `--render null` needs neither
GStreamer nor the backend.

## Gates

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features gst -- -D warnings
cargo build --release --features gst
```

There are no unit tests here: the behaviour this crate owns is argument parsing and the two
files, and the session itself is tested in the library.
