# parlor-cli

`parlor`: an AirPlay receiver on the command line.

It is the program, not the protocol. The receiver itself is
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) and playback is
[`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream); this crate
parses arguments, keeps two files on disk, and installs a logger.

## Building

```sh
cargo build --release --features gst
```

Without `--features gst` the program builds and plays nothing (`--render null`), which
reads a session and is how it is checked without a sound card. With it, GStreamer must be installed: the development files to
build, the runtime and its plugins to play. See the backend repository's README for the
element list per platform.

## Running

```sh
parlor -n "Living room" -pin 1234
```

A build with the `gst` feature plays through GStreamer by default, which is what a UxPlay
command expects; `--render null` is there for a session that should be read without sound.

```
parlor 0.1.0 - an AirPlay receiver

  Name      Living Room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

On an iPhone or iPad, open the AirPlay menu, pick the receiver and type the PIN the program
prints. Screen mirroring starts as soon as the client connects; audio follows whenever the
client plays something.

## Options

The names match UxPlay's command line, so a UxPlay command runs here unchanged for
everything this receiver can honour:

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
| `-p [n]` | upstream's fixed port layout, or a base port `n` |
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
| `-q` | only errors on the console |
| `-v` | show the session's records as well |
| `-d` | show everything, including protocol detail |
| `--log-level <level>` | `off`, `error`, `warn` (the default), `info`, `debug` or `trace` |
| `-h`, `-help` | print the usage text and exit |

A UxPlay option this receiver cannot honour is refused with the reason rather than
ignored: `-hls` and its tuning options, `-mp4`, `-p2p`, `-vsync`, `-async`, `-vrtp`,
`-artp`, `-v4l2`, `-bt709`, `-srgb`, `-fs`, `-f`, `-r`, `-nc`, `-nofreeze` and `-vc`.
A command that needs one of them has to fail where the user can see it, not play a session
that quietly is not what was asked for.

Everything also has a long name, for a script that prefers words: `--name`, `--port`,
`--no-pin`, `--password`, `--device-id`, `--seed`, `--log`, `--render`, `--audio-sink`,
`--video-sink`, `--video-parser`, `--video-decoder`, `--warm-video`, `--pace-video`,
`--width`, `--height`, `--refresh-rate`, `--max-fps`, `--overscanned`, `--hevc`,
`--features`, `--restrict`, `--allow`, `--block`, `--legacy-ports`, `--timing-port`,
`--udp-control-port`, `--data-port`, `--mirror-port`, `--nohold`, `--reset`,
`--audio-latency`, `--volume`, `--taper`, `--db`, `--fps-data`, `--cover-art`,
`--metadata`, `--dacp`, `--audio-codec`, `--key-file`, `--clients-file`,
`--ephemeral-seed`, `--random-password`, `--verbose` and `--base-port`.

By default the console shows one line at start and nothing else but warnings and errors; the
PIN is printed when it is drawn. `-v` adds the session's own records, `-d` adds protocol detail,
and `-q` leaves errors only. The request trace written with `--log` is unaffected by any of
them.

The program keeps two files in the local application data directory, `parlor/identity.seed`
and `parlor/clients`. The first is created on the first run and is what keeps the receiver's
identity stable; the second lists client keys, one 64-character hex key per line. Deleting
the identity makes every client pair again; deleting the client list keeps the identity and
forgets the clients. `-key` and `-reg` move those files, and `-key` with no file keeps the
identity in memory only.

## Latency

A mirror is judged by how quickly the picture follows the device. The fastest path is the
hardware one, which is the default where it exists: `d3d11h264dec` into `d3d11videosink` on
Windows. `--video-sink d3d11videosink` brings that decoder with it; naming a sink and a
decoder separately is how a hardware sink ends up fed by a software decoder.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).

中文：[README.zh-CN.md](README.zh-CN.md)。
