# parlor-cli

[![CI](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml)

`parlor`: an AirPlay receiver on the command line.

It is the program, not the protocol. The receiver is
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) and playback is
[`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream); this crate
parses arguments, keeps two files, installs a logger, and prints. It contains no protocol code.

## Building

```sh
cargo build --release --features gst
```

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
parlor 0.2.0 - an AirPlay receiver

  Name      Living Room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

On an iPhone or iPad, open the AirPlay menu, pick the receiver and enter the PIN when it
appears. Screen mirroring starts as soon as the client connects; audio follows when the client
plays something.

## The console

The banner is printed once. After that the console shows warnings and errors only; a PIN or a
drawn password is printed when it is needed, whatever the log level is.

| Flag | What it shows |
| --- | --- |
| `-q` | errors only |
| the default | warnings and errors |
| `-v`, `--verbose` | plus the session's records: streams opening, clocks, media statistics |
| `-d` | plus protocol detail |
| `--log-level <level>` | names a level directly: `off`, `error`, `warn`, `info`, `debug`, `trace` |

`--log <file>` appends a request trace to a file and is independent of all of them.

## Options

| Option | Meaning |
| --- | --- |
| `-n <name>` | the name clients see; defaults to `parlor` |
| `-nh` | accepted; this receiver never appends a hostname |
| `-h265` | allow HEVC and offer 4K |
| `-pin [nnnn]` | ask for a PIN, optionally a fixed one; this is the default |
| `-pw [password]` | require a password; without one, a random one is printed |
| `-m [mac]` | advertise this hardware address |
| `-key [file]` | where the identity seed lives; without a file nothing is stored |
| `-reg [file]` | where remembered clients live |
| `-p [n]` | the fixed media port layout, or a base port `n` |
| `-al <seconds>` | the audio delay a client schedules against; 0.25 by default |
| `-vol <0..1>` | the volume a session starts at |
| `-db <low:high>` | rescale the volume slider onto a dB range; `-30:0` by default |
| `-taper` | use the tapered volume profile |
| `-s <w>x<h>[@<r>]` | the display offered to a client; 1920x1080@60 by default |
| `-fps <fps>` | the highest frame rate asked of the client; 30 by default |
| `-o` | offer an overscanned display |
| `-as <sink>` / `-vs <sink>` | GStreamer audio and video sinks |
| `-vp <parser>` / `-vd <decoder>` | the parser and decoder the mirror pipeline uses |
| `-avdec` | force the software decoder |
| `-ca [file]` | report cover art, or write it to a file |
| `-md <file>` | write now-playing text to a file |
| `-dacp [file]` | export the remote-control identifiers |
| `-reset <seconds>` | close a session with no `/feedback` for n seconds; `0` never |
| `-nohold` | a new client takes over from the current one |
| `-restrict` / `-allow <id>` / `-block <id>` | which clients may connect |
| `-FPSdata` | log the client's own streaming reports |
| `-q`, `-v`, `-d`, `--log-level` | what the console shows |
| `-h`, `-help` | print the usage text and exit |

Everything also has a long name, for a script that prefers words: `--name`, `--port`,
`--no-pin`, `--password`, `--device-id`, `--seed`, `--log`, `--render`, `--audio-sink`,
`--video-sink`, `--video-parser`, `--video-decoder`, `--warm-video`, `--pace-video`,
`--width`, `--height`, `--refresh-rate`, `--max-fps`, `--overscanned`, `--hevc`,
`--features`, `--restrict`, `--allow`, `--block`, `--legacy-ports`, `--timing-port`,
`--udp-control-port`, `--data-port`, `--mirror-port`, `--nohold`, `--reset`,
`--audio-latency`, `--volume`, `--taper`, `--db`, `--fps-data`, `--cover-art`,
`--metadata`, `--dacp`, `--audio-codec`, `--key-file`, `--clients-file`,
`--ephemeral-seed`, `--random-password`, `--verbose` and `--base-port`.

### Port layout and passwords

- `-p n` lays out the ports as `tcp`/`udp` at `n..n+2`, with the control port at `n+1`. `-p`
  alone selects the fixed layout: mirror `7100`, control `7000`, timing `7011`, UDP control
  `6001`, data `6000`.
- `-pw` without a password draws one for the process and prints it.

## Files

The program keeps two files in the local application data directory: `parlor/identity.seed` and
`parlor/clients`.

- `identity.seed` is created on the first run and is what keeps the receiver recognisable
  across restarts. Deleting it makes every client pair again.
- `clients` lists verified client keys, one 64-character hex key per line. Deleting it keeps
  the identity and forgets the clients.

`-key` and `-reg` move those files; `-key` without a file keeps the identity in memory only, so
nothing is stored.

## Latency

A mirror is judged by how quickly the picture follows the device. The fastest path is the
hardware one, which is the default where it exists: `d3d11h264dec` into `d3d11videosink` on
Windows. `-vs d3d11videosink` brings that decoder with it; naming a sink and a decoder
separately is how a hardware sink ends up fed by a software decoder.

## Development

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --features gst -- -D warnings
```

The tests parse the command line: the option names, the port layout, the optional values and the
refusal of an invalid one. They need no GStreamer.

## Releasing

The `Release` workflow is started by hand with three inputs: a tag such as `v0.2.0`, the
release notes (leave empty to use `RELEASE_NOTES.md`; otherwise use `\n` for a line break),
and whether the release is a draft. It builds
`parlor` for Linux and Windows with the GStreamer backend, packs the binary with both
READMEs and the licence, and creates a GitHub release. The Windows build uses the GNU
toolchain that matches the MSYS2 GStreamer libraries; both archives need GStreamer 1.0 and
its base plugins on the machine that runs them.

## Documentation

- [rust-airplay's API](https://docs.rs/rust-airplay): `Config`, the renderer traits, the modules
- [rust-airplay's backend guide](https://github.com/rust-airplay/rust-airplay/blob/main/docs/BACKEND.md):
  how a playback backend is written
- [rust-airplay-gstream's README](https://github.com/rust-airplay/rust-airplay-gstream): sinks,
  elements and behaviour of the playback path
- [CHANGELOG.md](CHANGELOG.md): what each release contains

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).

Chinese: [README.zh-CN.md](README.zh-CN.md).
