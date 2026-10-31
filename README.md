# parlor-cli

[![CI](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml)

`parlor`: an AirPlay receiver on the command line.

It is the program, not the protocol. The receiver is
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) and playback is
[`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream); this crate
parses arguments, keeps two files, installs a logger, and prints. It contains no protocol code.

The run is also a library, named `parlor`, so a program can start a receiver without going
through the binary; `src/main.rs` only hands over to it. The pieces are the command line in
`src/cli.rs`, the meaning of a command line in `src/plan.rs`, the files and their writer in
`src/files.rs`, the banner and the log in `src/console.rs`, and the backend in `src/render.rs`.

New here? The [user guide](GUIDE.md) walks through installation, GStreamer and the first pairing.

**Contents:** [Requirements](#requirements) - [Building](#building) - [Running](#running) -
[The console](#the-console) - [Options](#options) - [Exit codes](#exit-codes) - [Files](#files) -
[Latency](#latency) - [Diagnosing a mirror](#diagnosing-a-mirror) -
[Known limitations](#known-limitations) - [Testing](#testing) - [Releasing](#releasing) -
[Documentation](#documentation)

## Requirements

- Rust 1.85 or newer to build; the MSRV is checked in CI.
- `--features gst` needs GStreamer 1.0 and its base plugins: the development files to build,
  the runtime and the plugins to play. A build without the feature needs no media library.
- **A network connection for the first build.** `rust-airplay` and `rust-airplay-gstream`
  are git dependencies pinned to a revision, not published versions, because the releases
  this program needs are ahead of crates.io. `Cargo.toml` carries the revisions; a build
  clones them once and then works offline.

  The pinned `rust-airplay` is the one that bounds the wait for a key frame after a queue
  overflow. Without it, a single overflow freezes the picture for the rest of the session;
  see [Latency](#latency) and [Diagnosing a mirror](#diagnosing-a-mirror).

## Building

```sh
cargo build --release
cargo build --release --features gst
```

The first build clones the two libraries at the revisions named in `Cargo.toml`; after that
`--offline` works and `--locked` keeps the graph exactly as tested.

Without `--features gst` the program builds and plays nothing: it reads a session and reports
it, which is how a session is checked without a sound card. With the feature, GStreamer must be
installed - the development files to build, and the runtime plus its plugins to play. The
backend repository's README lists the elements per platform.

## Running

```sh
parlor -n "Living room" -pin 1234
```

A build with the `gst` feature plays through GStreamer unless told otherwise. `--render null`
reads a session without playing it.

The program prints a short banner and then waits:

```
parlor 0.3.1 - an AirPlay receiver

  Name      Living room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

On an iPhone or iPad, open the AirPlay menu, pick the receiver and enter the PIN when it
appears. Screen mirroring starts as soon as the client connects; audio follows when the client
plays something.

`--bind <addr>` limits the address the receiver listens on; the default is every interface.
`--no-advertise` keeps the receiver off mDNS, so a client is not shown it and has to be told
where it is. `--version` prints the version and exits.

## The console

The banner is printed once, unless the log level hides warnings, as `-q` and `--log-level off`
do. After that the console shows warnings and errors only; a PIN or a drawn password is printed
when it is needed, whatever the log level is.

All log output goes to standard error. Standard output carries only what a person reads: the
banner, the PIN, a drawn password. A script can capture the diagnostics without losing the
banner, and a supervisor sees a warning and a fatal error on the same stream.

| Flag | What it shows |
| --- | --- |
| `-q`, `--quiet` | errors only |
| the default | warnings and errors |
| `-v`, `--verbose` | plus the session's records: streams opening, clocks, media statistics |
| `-d` | plus protocol detail |
| `--log-level <level>` | names a level directly: `off`, `error`, `warn`, `info`, `debug`, `trace` |

`-q` hides the banner with everything else: it prints errors only, and a banner is not an
error. A PIN or a drawn password is still printed, on standard output.

`--log <file>` appends a request trace to a file and is independent of all of them.

## Options

The short names are the familiar ones for an AirPlay receiver; where an option also has a long
spelling, for a script that prefers words, the table gives both.

| Option | Meaning |
| --- | --- |
| `-n <name>`, `--name <name>` | the name clients see; defaults to `parlor` |
| `-nh` | accepted; this receiver never appends a hostname |
| `-h265`, `--hevc` | allow HEVC, and offer 4K unless a size is given |
| `-pin [nnnn]`, `--pin [nnnn]` | ask for a PIN, optionally a fixed one; this is the default |
| `--no-pin` | pair with no PIN |
| `-pw [password]`, `--password <password>` | require a password; `-pw` without one draws it and prints it |
| `--random-password` | draw a password for the run and print it |
| `--password-file <path>` | read the password from a file, or from standard input for `-` |
| `-m [mac]`, `--device-id <mac>` | advertise this hardware address |
| `-key [file]`, `--key-file <file>` | where the identity seed lives; without a file nothing is stored |
| `--ephemeral-seed` | keep the identity for this run only |
| `--seed <hex>` | a fixed seed: 64 hexadecimal characters |
| `--seed-file <path>` | read the seed from this file |
| `-reg [file]`, `--clients-file <file>` | where remembered clients live |
| `--data-dir <path>` | the directory the identity and the client list live in |
| `-p [n]`, `--base-port <n>` | the fixed media port layout, or a base port `n` |
| `--legacy-ports` | the protocol's fixed media port layout |
| `--port <n>` | the control port; `0` lets the system choose |
| `--bind <addr>` | the address to listen on; the default is every interface |
| `--no-advertise` | do not announce the receiver over mDNS |
| `--timing-port <n>`, `--udp-control-port <n>`, `--data-port <n>`, `--mirror-port <n>` | move one media port off its layout |
| `-al <seconds>`, `--audio-latency <seconds>` | the audio delay a client schedules against; 0.25 by default |
| `-vol <0..1>`, `--volume <0..1>` | the volume a session starts at |
| `-db <low:high>`, `--db <low:high>` | rescale the volume slider onto a dB range; `-30:0` by default |
| `-taper`, `--taper` | use the tapered volume profile |
| `-s <w>x<h>[@<r>]` | the display offered to a client; 1920x1080@60 by default |
| `--width <px>`, `--height <px>`, `--refresh-rate <hz>` | the same display, stated in parts |
| `-fps <fps>`, `--max-fps <fps>` | the highest frame rate asked of the client; 30 by default |
| `-o`, `--overscanned` | offer an overscanned display |
| `-as <sink>`, `--audio-sink <sink>` | the GStreamer audio sink |
| `-vs <sink>`, `--video-sink <sink>` | the GStreamer video sink |
| `-vp <parser>`, `--video-parser <parser>` | the parser the mirror pipeline uses |
| `-vd <decoder>`, `--video-decoder <decoder>` | the decoder an H.264 mirror stream uses |
| `--video-decoder-hevc <decoder>` | the decoder an H.265 mirror stream uses |
| `-avdec` | force the software decoder |
| `--warm-video` | keep the video pipeline between sessions, so the next connection does not pay for a window |
| `--pace-video` | hold mirror frames to their capture spacing; off by default, and costs at most 40 ms a frame |
| `--no-pace-video` | name the default explicitly: frames go as fast as the renderer takes them |
| `--render <backend>` | what plays the media: `null` or `gst` |
| `--audio-codec <coding>` | the coding a session falls back to: `alac` or `aac-eld` |
| `-ca [file]`, `--cover-art [file]` | report each image's type and size on standard error, or write it to a file |
| `-md <file>`, `--metadata <file>` | write the now-playing text to a file |
| `-dacp [file]`, `--dacp <file>` | export the remote-control identifiers |
| `-reset <seconds>`, `--reset <seconds>` | close a session with no `/feedback` for n seconds; `0` never |
| `-nohold`, `--nohold` | a new client takes over from the current one |
| `-restrict`, `--restrict` | admit only the devices named with `-allow` |
| `-allow <id>`, `--allow <id>` | a device that may connect; repeatable |
| `-block <id>`, `--block <id>` | a device that never may; repeatable |
| `-FPSdata`, `--fps-data` | log the client's own streaming reports |
| `-q`, `--quiet` | errors only |
| `-v`, `--verbose` | warnings, errors and the session's records |
| `-d` | everything, including protocol detail |
| `--log-level <level>` | `off`, `error`, `warn`, `info`, `debug` or `trace` |
| `--log <file>` | append a request trace to a file |
| `-V`, `--version` | print the version and exit |
| `-h`, `-help`, `--help` | print the usage text and exit |

### Port layout and passwords

- `-p n` lays out the ports as `tcp`/`udp` at `n..n+2`, with the control port at `n+1`. `-p`
  alone selects the fixed layout: mirror `7100`, control `7000`, timing `7011`, UDP control
  `6001`, data `6000`. `--port` moves the control port on its own, and `--timing-port`,
  `--udp-control-port`, `--data-port` and `--mirror-port` move one media port each.
- `-pw` without a password draws one for the process and prints it. `--password-file` reads one
  from a file, or from standard input when the path is `-`, and drops the line ending; a
  password needs at least four characters. A drawn password is printed on standard output.

### Client lists

`-allow` and `-block` take a device identifier and normalise it to twelve upper-case
hexadecimal digits, so `aa:bb:cc:dd:ee:ff`, `11-22-33-44-55-66` and `aabbccddeeff` name the
same device; a value that is not twelve hex digits is refused rather than silently matching
nothing. `-allow` alone turns `-restrict` on and prints a warning, because the library ignores
the list without it; `-restrict` with no `-allow` refuses every device and says so.

### Conflicts and values

A command line that says two contradictory things is refused with exit code 2 rather than
resolved silently:

- the identity comes from exactly one of `--seed`, `--seed-file`, `-key <file>` and
  `-key`/`--ephemeral-seed`;
- `-h265` and `--features` both set the advertised features;
- the password is set by one of `-pw`, `--password`, `--random-password` and `--password-file`;
- `-p <base>` sets the whole port layout, so it cannot be combined with `--port` or with
  `--timing-port`, `--udp-control-port`, `--data-port` or `--mirror-port`.

An option that takes a value refuses to take the next option for it: `-n -p 7000`,
`-allow -restrict` and `-md -q` report that a value is needed instead of consuming the option.
A numeric option still accepts a value that begins with a dash, so `-db -30:0` is a range.

Every number is checked where it is read: `-vol`, `--volume`, `-al`, `--audio-latency` and
`-reset` refuse NaN and infinity, `-s` takes at most 16384 pixels on either side and a
refresh rate of 1 to 240, `-fps` and `--max-fps` take 1 to 240, and `-reset` takes at most
86400 seconds.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | the run succeeded, and so did `--help` and `--version` |
| `1` | a run that could not start or that failed: an identity that cannot be read, a password file that cannot be read, a backend this build or this machine does not have |
| `2` | a command line this program cannot accept: an unknown option, an option with no value, a value outside its range, or two options that contradict each other |

A command line that cannot be accepted is reported on standard error and exits 2; an unknown
or unsupported option also prints the usage text. A run that fails prints one line naming what
went wrong and exits 1.

## Files

The program keeps two files in the local application data directory: `parlor/identity.seed`
and `parlor/clients`. The directory is `%LOCALAPPDATA%\parlor` on Windows, and
`$XDG_DATA_HOME/parlor` or `$HOME/.local/share/parlor` on Unix; `--data-dir <path>` names it
instead, and a machine with none of those variables set needs that option.

- `identity.seed` is created on the first run and is what keeps the receiver recognisable
  across restarts. Deleting it makes every client pair again. On Unix it is created readable
  by its owner alone (0600). It is only created when it does not exist: a seed that exists and
  cannot be read is reported and the run stops, because replacing it would silently drop every
  pairing.
- `clients` lists verified client keys, one 64-character hex key per line. Deleting it keeps
  the identity and forgets the clients. A client is appended only when it is new, so a client
  that reconnects does not grow the file.

`-key` and `-reg` move those files; `-key` without a file keeps the identity in memory only, so
nothing is stored, and the client registry stays in memory too unless `-reg` names a file.

Writes a client can cause - cover art, now-playing text, remote-control identifiers - go
through one writer thread with a queue of sixteen. A full queue drops the write and says so
rather than holding up the session.

## Latency

A mirror is judged by how quickly the picture follows the device. Two things spend it, and
they are worth telling apart.

**The renderer's queue.** Frames go to the renderer as fast as it takes them: this program
holds nothing back by default, and a renderer that keeps up leaves the queue empty, which is
the shortest path from the device to the screen.

The queue holds eight frames, and if it does fill - a renderer that cannot keep up, or a
stream that arrives in bursts - `rust-airplay` 0.3.1 clears it and waits for the next key
frame rather than dropping one frame of a group. A frame that a later frame refers to cannot be
dropped without damaging the picture, so the cost of falling behind is a short freeze instead
of a corrupted one.

`--pace-video` is the other side of that trade: each frame is held back to the spacing it was
captured at, at most 40 ms per frame. It spends a little latency to smooth a stream that
arrives in bursts, and it does nothing at all when the stream is already evenly spaced.

**The decoder.** Since the 0.3 backend a sink and a decoder are separate choices, so
`-vs d3d11videosink` no longer brings `d3d11h264dec` with it. A run that names a hardware sink
but no decoder decodes every frame on the CPU (`avdec_h264`) and has less headroom, which is
the queue above filling up once the picture gets busy.

```sh
parlor -vs d3d11videosink -vd d3d11h264dec
```

An HEVC stream (`-h265`) needs its decoder named separately, because one hardware element
decodes one coding:

```sh
parlor -h265 -vs d3d11videosink -vd d3d11h264dec --video-decoder-hevc d3d11h265dec
```

A run without `-vd` says so at startup.

## Diagnosing a mirror

`-v` prints what the receiver thinks of its own stream. Four lines answer most questions:

```text
render    h264   fdsrc ! h264parse ! d3d11h264dec ! d3d11videosink sync=false
media     mirror: 308 packets, 3946087 bytes in, 297 received, 297 written, 1 waiting, 0 dropped
media     mirror summary: 1204 received, 1204 written, 0 waiting, 0 dropped
media     pace frame 12: queued 3 ms, pacer wanted 31 ms, waited 0 ms
```

| What it says | What it means | What to do |
| --- | --- | --- |
| `received` and `written` keep up, `waiting` 0 or 1, `dropped` 0 | healthy | nothing |
| `waiting` at 8, `dropped` climbing | the renderer cannot keep up | name the hardware decoder with `-vd`; or ask for a smaller picture with `-s 1280x720`; or a lower `-fps` |
| `queued` in the hundreds of milliseconds | frames wait in the queue before the renderer | the same as above |
| `h264 ... avdec_h264` in the pipeline line | video is decoded on the CPU | `-vd d3d11h264dec`, or whatever decoder the machine has |
| `the renderer fell behind` | the queue was cleared; frames are dropped until the next key frame | the same as above |
| `written` stops climbing while `received` keeps going, picture frozen | the key frame the receiver is waiting for never comes (see the known issue in [Requirements](#requirements)) | reconnect; the fix belongs in the library |

`-FPSdata` adds the client's own streaming report, which says what the device thinks of the
connection - the one thing the receiver cannot measure itself.

## Known limitations

A build started with `-ca`, `-md` or `-dacp` accepts writes and terminal output that a client
asks for, and the protocol does not authenticate that request: any host on the local network
can send a `SET_PARAMETER` request and cause a file to be written or a line to be printed. The
program bounds what such a request can do - control characters are stripped from anything
printed, a printed listing is cut at 2 KiB, cover art over 4 MiB and metadata over 256 KiB are
dropped, and the writes go through a bounded queue - but a real fix has to authenticate the
request in the library. Until then, treat the files these options name as public, and point
them at a path that is not worth protecting.

## Testing

```sh
cargo test
cargo test --features gst
cargo fmt --all -- --check
cargo clippy --all-targets --features gst -- -D warnings
```

`src/` holds 34 unit tests: the parser, the display and port plans, the seed and the file
writer, the console's printable text, and the redaction of a secret in a log record. `tests/cli.rs` holds seven integration tests that run
the built binary through `CARGO_BIN_EXE_parlor`: the exit codes, the option list in `--help`,
`--version`, a seed that is not hexadecimal, a closed output pipe, a backend this build does
not have, and a receiver that starts and keeps its identity. The one that asks for the
GStreamer backend only runs in a build without the feature, so `cargo test --features gst` runs
six of them. None of the tests need GStreamer.

## Releasing

The `Release` workflow is started by hand with three inputs: a tag such as `v0.3.2` (a
missing `v` is added), the release notes (leave empty to use `RELEASE_NOTES.md`; otherwise use
`\n` for a line break), and whether the release is a draft. It builds `parlor` for Linux and
Windows with the GStreamer backend, packs the binary with the README and the licence, and
creates a GitHub release. The Windows build uses the GNU toolchain that matches the MSYS2
GStreamer libraries; both archives need GStreamer 1.0 and its base plugins on the machine that
runs them.

## Documentation

- [rust-airplay's API](https://docs.rs/rust-airplay): `Config`, the renderer traits, the modules
- [rust-airplay's backend guide](https://github.com/rust-airplay/rust-airplay/blob/main/docs/BACKEND.md):
  how a playback backend is written
- [rust-airplay-gstream's README](https://github.com/rust-airplay/rust-airplay-gstream): sinks,
  elements and behaviour of the playback path
- [GUIDE.md](GUIDE.md): installing, GStreamer, every option, troubleshooting
- [CHANGELOG.md](CHANGELOG.md): what each release contains
- [MIGRATION.md](MIGRATION.md): what changed between releases, and what to do about it

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).

Chinese: [README.zh-CN.md](README.zh-CN.md).
