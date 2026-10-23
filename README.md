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

Without `--features gst` the program builds and runs with `--render null`, which reads a
session and plays nothing. With it, GStreamer must be installed: the development files to
build, the runtime and its plugins to play. See the backend repository's README for the
element list per platform.

## Running

```sh
parlor --name "Living room" --port 7000 --render gst
```

On an iPhone or iPad, open the AirPlay menu, pick the receiver and type the PIN the program
prints. Screen mirroring starts as soon as the client connects; audio follows whenever the
client plays something.

## Options

| Option | Meaning |
| --- | --- |
| `--name <name>` | the name clients see; defaults to `parlor` |
| `--port <port>` | control port; defaults to 7000, `0` picks a free one |
| `--pin [<digits>]` | ask for a PIN, optionally a fixed one; this is the default |
| `--no-pin` | do not ask for a PIN |
| `--password <password>` | also require the digest password at `SETUP` |
| `--device-id <address>` | advertise this hardware address |
| `--seed <hex>` | 32 bytes of identity, as 64 hex characters |
| `--log <file>` | append the request trace to a file |
| `--render <null\|gst>` | what plays the media; `null` by default |
| `--audio-sink <name>` | GStreamer audio sink |
| `--video-sink <name>` | GStreamer video sink; changing it also changes the decoder |
| `--video-parser <name>` | the parser the mirror pipeline starts with |
| `--video-decoder <name>` | the decoder that follows it |
| `--warm-video` | start the video pipeline with the receiver |
| `--pace-video` | hold video to its capture spacing, for even frames at some latency |
| `--width <pixels>` / `--height <pixels>` | the display offered to a client; 1920x1080 by default |
| `--refresh-rate <hz>` | the refresh rate offered; 60 by default |
| `--max-fps <fps>` | the highest frame rate asked of the client; 30 by default |
| `--overscanned` | offer an overscanned display |
| `--audio-latency <seconds>` | the audio delay a client schedules against; 0.25 by default |
| `--volume <0..1>` | the volume a session starts at; full by default |
| `--audio-codec <alac\|aac-eld>` | the coding a client sends; `aac-eld` by default |
| `--help` | print the usage text and exit |

The program keeps two files in the local application data directory, `parlor/identity.seed`
and `parlor/clients`. The first is created on the first run and is what keeps the receiver's
identity stable; the second lists client keys, one 64-character hex key per line. Deleting
the identity makes every client pair again; deleting the client list keeps the identity and
forgets the clients.

## Latency

A mirror is judged by how quickly the picture follows the device. The fastest path is the
hardware one, which is the default where it exists: `d3d11h264dec` into `d3d11videosink` on
Windows. `--video-sink d3d11videosink` brings that decoder with it; naming a sink and a
decoder separately is how a hardware sink ends up fed by a software decoder.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).

中文：[README.zh-CN.md](README.zh-CN.md)。
