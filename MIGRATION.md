# Migration

中文见 [MIGRATION.zh-CN.md](MIGRATION.zh-CN.md)。

Three migrations are described here: the split of this program out of the library's workspace,
which made it 0.2.0, the command line changes of 0.3.0, and the library fix of 0.3.1.

## 0.3.0 to 0.3.1

Nothing to change in a configuration; this release is about the libraries underneath.

1. **Move the dependencies to 0.3.1.** `rust-airplay` and `rust-airplay-gstream` 0.3.0 carry a
   pacing defect: the receiver reported each frame's arrival interval to the backend, which
   slept that interval before writing, so the renderer ran permanently behind the stream and its
   frame queue never drained. A mirror on 0.3.0 stutters, and no option fixes it.
2. **Drop `--pace-video` from a command line that only carried it as a workaround.** It is off
   by default again, which is the shortest path from the device to the screen. It is still
   useful for a stream that arrives in bursts; it costs at most 40 ms a frame.
3. **Nothing else changes.** The option set, the exit codes, the files and their formats are the
   same as 0.3.0.

## 0.2.0 to 0.3.0

The receiver itself is the same: the default data directory, the option names and the two file
formats did not change. What changed is the command line's edge cases, the exit codes, and how
the code is arranged.

### What to do

1. **Move the dependencies to 0.3.** The two libraries are published on crates.io; there is no
   Git or path dependency to update first, and no branch to track. A lock file pinned to the
   0.2 line has to be regenerated.
2. **Teach your scripts the exit codes.** A command line this program cannot accept now exits
   2 instead of 1. Anything that treated any non-zero code as `the receiver failed to start`
   has to distinguish 2 (fix the command line) from 1 (look at the receiver). 0 still means
   success, and `--help` and `--version` are successes.
3. **Look for command lines that now conflict.** These used to be resolved silently and are
   refused now, with exit code 2:
   - two identity sources among `--seed`, `--seed-file`, `-key <file>` and `-key`;
   - `-h265` together with `--features`;
   - a password from `-pw`, `--password` or `--random-password` together with
     `--password-file`;
   - `-p <base>` together with `--port` or with `--timing-port`, `--udp-control-port`,
     `--data-port` or `--mirror-port`.
4. **Check `-allow` on its own.** `-allow` without `-restrict` now turns `-restrict` on and
   prints a warning. A receiver that was started with an allow list and no `-restrict` used to
   admit everyone; it now admits only the listed devices. If the old behaviour was intended,
   name the devices to refuse with `-block` instead, or drop the list.
5. **Check the identifiers you pass.** `-allow` and `-block` take twelve hex digits, and
   normalise the separators: `aa:bb:cc:dd:ee:ff`, `11-22-33-44-55-66` and `aabbccddeeff` are
   the same device. A value that is not twelve hex digits is an error now, where it used to be
   accepted and then match nothing.
6. **Move a stdout reader to stderr if it read warnings.** All log output goes to standard
   error. Standard output carries only the banner, the PIN and a drawn password.
7. **Check for NaN and infinity in generated arguments.** `-vol`, `--volume`, `-al`,
   `--audio-latency` and `-reset` refuse them, along with `-s` sizes over 16384 and refresh
   rates outside 1 to 240, and `-reset` values over 86400 seconds. A generated command line
   that used to pass such a value now exits 2.
8. **Check `-h265` together with a stated size.** `-h265` enlarges the default display to 4K
   only when no size was given. `-h265 -s 1920x1080` keeps 1080p; a command that relied on the
   old unconditional enlargement has to state the 4K size itself.
9. **If you read `-ca` output, read stderr.** `-ca` without a path reports each image's
   content type and byte count on standard error instead of doing nothing. Nothing is written
   to disk, so a pipeline that only watches files is unaffected.

### A damaged identity file

0.3.0 creates `identity.seed` only when the file does not exist, and stops with exit code 1
when the file exists and cannot be read. It will not quietly give you a new identity, because
that would drop every pairing the old one had.

If the program stops with `parlor: cannot read <path>: ...` and exit code 1, work through
these in order:

1. Check the path exists and is a file. If `--data-dir`, `-key` or `--key-file` names it, the
   path in the message is the one that was used.
2. Check the account that runs `parlor` can read it. On Unix, `ls -l` should show the user as
   owner with mode 0600; a seed copied from another account may be owned by that account.
3. Check the contents: the file is 32 bytes written as 64 hexadecimal characters. A text
   editor that saved it as UTF-16, or a manual edit that left 63 or 65 characters, is the most
   common damage.
4. If you have a backup, restore it. That is the only way to keep the old pairings.
5. If you accept re-pairing, move the damaged file aside and start the receiver again. A new
   identity is created; every client pairs again. The `clients` file can stay, but its keys
   belong to the old identity.

Windows:

```powershell
Move-Item "$env:LOCALAPPDATA\parlor\identity.seed" "$env:LOCALAPPDATA\parlor\identity.seed.broken"
```

Unix:

```sh
mv "${XDG_DATA_HOME:-$HOME/.local/share}/parlor/identity.seed" \
   "${XDG_DATA_HOME:-$HOME/.local/share}/parlor/identity.seed.broken"
```

Do not delete the file and do not point `--data-dir` at an empty directory unless losing the
pairings is what you want.

### Dependencies

```toml
rust-airplay = "0.3"
rust-airplay-gstream = { version = "0.3", optional = true }
```

Both are published on crates.io. There is no Git dependency and no path dependency: this
repository builds from a checkout without the two libraries' repositories beside it, as long
as crates.io (or a mirror, or a vendor directory) can supply the two releases. The `gst`
feature is off by default, and `--render null` needs neither GStreamer nor the backend.

### Code layout

`src/main.rs` no longer holds the program. It is the entry point, and everything it used to do
is in the `parlor` library:

| Before | Now |
| --- | --- |
| one `src/main.rs` | `src/lib.rs` plus `cli`, `plan`, `files`, `console`, `render` |
| private functions in a binary | `pub` modules of a library named `parlor` |
| no library target | `[lib] name = "parlor"` next to the `parlor` binary |

A build that only runs the binary needs no change: the binary is still named `parlor`.

## The program becomes its own repository (0.1 to 0.2.0)

This crate was `crates/rust-airplay-cli` inside the library's workspace, producing a binary
called `rust-airplay`. It is the `parlor-cli` repository now, producing `parlor`.

### What changed

| Before | Now |
| --- | --- |
| package `rust-airplay-cli` | package `parlor-cli` |
| binary `rust-airplay` | binary `parlor` |
| default receiver name `rust-airplay` | default receiver name `parlor` |
| data directory `rust-airplay/` | data directory `parlor/` |
| path dependencies | published crates from crates.io |
| `rust_airplay_gstreamer::` | `rust_airplay_gstream::` |

At the split the two libraries were Git dependencies on `main`; 0.2.0 and 0.3.0 use the
published releases, so the Git phase is over.

The data directory changed name, so an existing pairing does not carry over. If you want to
keep the identity, move the directory:

```powershell
Move-Item "$env:LOCALAPPDATA\rust-airplay" "$env:LOCALAPPDATA\parlor"
```

## Gates

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features gst -- -D warnings
cargo test
cargo test --features gst
cargo build --release --features gst
```

There are tests here now, and they do not need GStreamer. `src/` holds 33 unit tests: the
argument parser, the display and port plans, the seed and file writer, and the console's
printable text. `tests/cli.rs` holds seven integration tests that run the built binary through
`CARGO_BIN_EXE_parlor`: the exit codes, the option list in `--help`, `--version`, a seed that
is not hexadecimal, a closed output pipe, a backend this build does not have, and a receiver
that starts and keeps its identity. The one that asks for the GStreamer backend only applies
to a build without the feature, so `cargo test --features gst` runs six of them.
