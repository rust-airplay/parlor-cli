# parlor user guide

中文见 [GUIDE.zh-CN.md](GUIDE.zh-CN.md)。

`parlor` is an AirPlay receiver for Windows and Linux. It advertises itself on the local
network, pairs with an iPhone, iPad or Mac, and plays screen mirroring and audio through
GStreamer. Everything that speaks the protocol lives in
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) and
[`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream); `parlor` is
the program around them: the command line, the files, the console.

This guide is the practical one: how to install it, how to get GStreamer right, what every
option does, and what to look at when there is no picture. The shorter
[README](README.md) is the reference table.

**Contents**

1. [Installing](#1-installing)
2. [GStreamer](#2-gstreamer)
3. [First run and pairing](#3-first-run-and-pairing)
4. [Everyday use](#4-everyday-use)
5. [Options](#5-options)
6. [Recipes](#6-recipes)
7. [Files](#7-files)
8. [Exit codes](#8-exit-codes)
9. [Troubleshooting](#9-troubleshooting)
10. [Security](#10-security)

## 1. Installing

### From an archive

A release archive contains the binary, the README and the licence:

- Windows: `parlor-v0.3.2-x86_64-pc-windows-gnu.zip` - unzip it, run `parlor.exe`.
- Linux: `parlor-v0.3.2-x86_64-unknown-linux-gnu.tar.gz` - extract it, run `./parlor`.

Both are built with the GStreamer backend, so **GStreamer is a prerequisite** (section 2). The
archives do not bundle it.

### From source

```sh
git clone https://github.com/rust-airplay/parlor-cli
cd parlor-cli
cargo build --release --features gst     # plays video and audio
cargo build --release                    # reads a session, plays nothing
```

Rust 1.85 or newer. The two libraries are git dependencies pinned to a revision in
`Cargo.toml`, so the first build clones them; after that `cargo build --offline` works. Use
`--locked` to build exactly the graph the release was tested with.

The binary lands in `target/release/parlor` (`parlor.exe` on Windows).

### Checking the build

```sh
./parlor --version          # parlor 0.3.2
./parlor --help             # every option
```

## 2. GStreamer

The `gst` feature is what turns `parlor` from a session reader into a player. Without it,
`--render null` is all you get: the receiver accepts a session, decrypts and counts the
media, and plays nothing. That is useful for testing a network or a pairing without a sound
card, and it is why the feature is optional.

### Plugins this program actually uses

| Element | Package | Used for |
| --- | --- | --- |
| `h264parse`, `h265parse` | gst-plugins-base | splitting the mirror stream into frames |
| `avdec_h264`, `avdec_h265` | gst-libav | software decoding (the fallback) |
| `d3d11h264dec`, `d3d11h265dec`, `d3d11videosink` | gst-plugins-bad (Windows) | hardware decoding and the video window |
| `vah264dec`, `vah265dec` | gst-plugins-bad (Linux, VA-API) | hardware decoding |
| `wasapisink` | gst-plugins-base (Windows) | audio |
| `pulsesink`, `alsasink` | gst-plugins-good/base (Linux) | audio |
| `autovideosink`, `glimagesink` | gst-plugins-base | the video window, when the hardware sink is absent |

### Windows

Install the **MSVC 64-bit** runtime and development installers of GStreamer 1.0 from
<https://gstreamer.freedesktop.org/download/> ("Complete" install), then make the runtime
findable:

```powershell
# A new terminal is needed after this
setx PATH "%PATH%;C:\gstreamer\1.0\msvc_x86_64\bin"
```

If you installed it somewhere else, point `GST_PLUGIN_PATH_1_0` at its `lib\gstreamer-1.0`
directory instead of adding it to `PATH`.

The release archive is built with the GNU toolchain against **MSYS2 UCRT64** GStreamer. If you
would rather match that exactly:

```sh
pacman -S mingw-w64-ucrt-x86_64-gstreamer mingw-w64-ucrt-x86_64-gst-plugins-base \
          mingw-w64-ucrt-x86_64-gst-plugins-bad mingw-w64-ucrt-x86_64-gst-libav
```

Either runtime works; the DLLs are found at run time, not linked in.

### Linux

```sh
# Debian, Ubuntu
sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
                 gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-tools

# Fedora
sudo dnf install gstreamer1-plugins-base gstreamer1-plugins-good \
                 gstreamer1-plugins-bad-free gstreamer1-libav gstreamer1-plugins-bad-freeworld
```

Building it needs the development files as well:

```sh
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

Hardware decoding on Linux goes through VA-API: install `gstreamer1.0-vaapi` and make sure the
kernel driver is loaded (`vainfo` is the usual check).

### Checking the install

```sh
gst-inspect-1.0 --version
gst-inspect-1.0 h264parse        # must exist
gst-inspect-1.0 d3d11h264dec     # Windows only
gst-inspect-1.0 vah264dec        # Linux only
gst-launch-1.0 --version
```

Then ask `parlor` itself: `./parlor --render gst -v` prints the pipelines it will build, or
exits with the reason it cannot.

### Environment variables

| Variable | Effect |
| --- | --- |
| `AIRPLAY_GST_LAUNCH` | the program used for a video pipeline, when `gst-launch-1.0` is not on `PATH` |
| `AIRPLAY_GST_AUDIO_SINK` | default audio sink, when `-as` is not given |
| `AIRPLAY_GST_VIDEO_SINK` | default video sink, when `-vs` is not given |
| `AIRPLAY_GST_VIDEO_WARM` | `1` keeps a video pipeline between sessions |
| `AIRPLAY_GST_KEEP_STDERR` | `1` keeps the pipeline's own output, for diagnosing a picture |
| `AIRPLAY_DUMP_VIDEO` | write the first video frames to this path as Annex-B |
| `AIRPLAY_DUMP_AUDIO` | write the audio payloads to this path |

## 3. First run and pairing

```sh
./parlor -n "Living room" -pin 1234
```

You should see:

```text
parlor 0.3.2 - an AirPlay receiver

  Name      Living room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

On the iPhone or iPad: Control Center, **Screen Mirroring**, choose the receiver, and type the
PIN if one is asked for. With `-pin 1234` the PIN is fixed; without a value
(`-pin`) the receiver draws a one-time PIN and prints it when a client asks:

```text
*** enter PIN 4821 on the client ***
```

A password works the same way and gates `SETUP`: `-pw` draws one and prints it,
`-pw hunter2` fixes it. A password needs at least four characters.

A client that paired once is remembered in the `clients` file and does not need the PIN
again. The identity in `identity.seed` is what makes the receiver recognisable across restarts;
delete it and every device pairs again.

## 4. Everyday use

- **Mirroring** starts when the device starts it; the video window appears then and closes
  when mirroring stops.
- **Audio** follows the session; `-vol` sets the level a session starts at, `-db` and
  `-taper` change how the slider maps onto it.
- **Stopping**: Ctrl+C. Nothing is written on the way out except a session summary in the log.
- **Several sessions**: a second client is served as well; `-nohold` makes a new one take over
  from the current one instead.
- **Choosing the pipeline**: `-vd` names the H.264 decoder, `--video-decoder-hevc` the H.265
  one, `-vs` the sink. Since 0.3 a sink no longer brings a decoder with it, so a hardware sink
  fed by the default decoder decodes on the CPU. On Windows that is usually
  `-vs d3d11videosink -vd d3d11h264dec`.
- **Smoothing a bursty stream**: `--pace-video` holds each frame to its capture spacing, at
  most 40 ms per frame. It is off by default; a stream that arrives evenly does not need it.

## 5. Options

Short names first, the long spelling in the same row. Every value is checked where it is read;
a value outside its range is an error, not a clamp.

### Identity and access

| Option | Meaning |
| --- | --- |
| `-n <name>`, `--name <name>` | the name clients see; `parlor` by default |
| `-nh` | accepted for compatibility; this receiver never appends a hostname |
| `-pin [nnnn]`, `--pin [nnnn]` | ask for a PIN; without a value it is a one-time PIN |
| `--no-pin` | pair with no PIN |
| `-pw [password]`, `--password <password>` | require a password; `-pw` without one draws and prints it |
| `--random-password` | draw a password for this run and print it |
| `--password-file <path>` | read the password from a file, or from standard input for `-` |
| `-key [file]`, `--key-file <file>` | where the identity seed lives; without a file, nothing is stored |
| `--seed <hex>` | a fixed seed: 64 hexadecimal characters |
| `--seed-file <path>` | read the seed from this file |
| `--ephemeral-seed` | a random seed for this run only |
| `-reg [file]`, `--clients-file <file>` | where remembered clients live |
| `--data-dir <path>` | the directory the identity and the client list live in |
| `-m [mac]`, `--device-id <mac>` | the hardware address to advertise |
| `-restrict`, `--restrict` | admit only the devices named with `-allow` |
| `-allow <id>`, `--allow <id>` | a device that may connect; repeatable |
| `-block <id>`, `--block <id>` | a device that never may; repeatable |

Device identifiers are normalised to twelve hexadecimal digits, so `aa:bb:cc:dd:ee:ff`,
`11-22-33-44-55-66` and `aabbccddeeff` name the same device; anything else is refused.
`-allow` on its own turns `-restrict` on and says so. These lists compare what a client
*claims* to be, so they are a convenience, not a security boundary: the PIN and the password
are the gate.

### Network and ports

| Option | Meaning |
| --- | --- |
| `--port <n>` | the control port; 7000 by default, `0` lets the system choose |
| `--bind <addr>` | the address to listen on; every interface by default |
| `--no-advertise` | do not announce the receiver over mDNS |
| `-p` | the protocol's fixed media port layout |
| `-p <n>`, `--base-port <n>` | that layout starting at `n`: timing `n`, control `n+1`, data `n+2`, mirror `n` (TCP) |
| `--legacy-ports` | the fixed layout: UDP 7011 timing, 6001 control, 6000 data, TCP 7100 mirror |
| `--timing-port <n>`, `--udp-control-port <n>`, `--data-port <n>`, `--mirror-port <n>` | move one media port off its layout |
| `--nohold` | a new client takes over from the current one |
| `--reset <seconds>` | close a session whose client stops sending `/feedback`; 0 never closes it (the default), at most 86400 |

Without a base port the media ports are chosen by the system and reported to the client in
`SETUP`, so nothing has to be forwarded. `--port 0` plus `--no-advertise` is a receiver
only a client that is told the address can reach.

### Display and video

| Option | Meaning |
| --- | --- |
| `--render <null|gst>` | what plays the media: `null` reads and plays nothing, `gst` hands it to GStreamer (the default in a build with the feature) |
| `-s <w>x<h>[@<r>]` | the display offered to a client; 1920x1080@60 by default, at most 16384 pixels each way and a refresh of 1 to 240 |
| `--width <px>`, `--height <px>`, `--refresh-rate <hz>` | the same display, stated in parts |
| `-fps <fps>`, `--max-fps <fps>` | the highest frame rate asked of the client; 30 by default, 1 to 240 |
| `-o`, `--overscanned` | offer an overscanned display |
| `-h265`, `--hevc` | allow HEVC; offers 4K unless a size is given |
| `--features <hex>` | advertise this feature bitmap instead of the default |
| `-as <sink>`, `--audio-sink <sink>` | the GStreamer audio sink |
| `-vs <sink>`, `--video-sink <sink>` | the GStreamer video sink |
| `-vp <parser>`, `--video-parser <parser>` | the parser for the mirror stream |
| `-vd <decoder>`, `--video-decoder <decoder>` | the decoder for an H.264 stream |
| `--video-decoder-hevc <decoder>` | the decoder for an H.265 stream |
| `-avdec` | force the software decoder for both codings |
| `--warm-video` | keep the video pipeline between sessions |
| `--pace-video`, `--no-pace-video` | hold frames to their capture spacing (off by default) |

A sink and a decoder are two choices: `-vs d3d11videosink` alone leaves the software decoder
in place. A run without `-vd` says so at startup.

### Audio

| Option | Meaning |
| --- | --- |
| `-al <seconds>`, `--audio-latency <seconds>` | the delay a client schedules its audio against; 0.25 by default, 0 to 10 |
| `-vol <0..1>`, `--volume <0..1>` | the volume a session starts at |
| `-db <low:high>`, `--db <low:high>` | rescale the volume slider onto a decibel range; `-30:0` by default |
| `-taper`, `--taper` | use the tapered volume profile |
| `--audio-codec <alac|aac-eld>` | the coding a session falls back to when the client does not say |

`-al` is a deliberate delay, not a defect: a client plays audio that much behind the video to
keep them together. Lower it (`-al 0.05`) for a snappier session, raise it if audio drops out.

### Files a client can write

| Option | Meaning |
| --- | --- |
| `-ca [file]`, `--cover-art [file]` | write cover art to this file; without one, report its type and size |
| `-md <file>`, `--metadata <file>` | write the now-playing listing to this file |
| `-dacp [file]`, `--dacp [file]` | export the remote-control identifiers; without a file, use the default path |

See [Security](#10-security) before you point any of these at something that matters.

### Logging and diagnostics

| Option | Meaning |
| --- | --- |
| `-q`, `--quiet` | errors only |
| the default | warnings and errors |
| `-v`, `--verbose` | plus the session's records |
| `-d` | plus protocol detail |
| `--log-level <l>` | `off`, `error`, `warn`, `info`, `debug`, `trace` |
| `--log <file>` | append a request trace to this file |
| `-FPSdata`, `--fps-data` | log the client's own streaming reports |
| `-V`, `--version` | print the version |
| `-h`, `-help`, `--help` | print the usage text |

Logs go to standard error. Standard output keeps the banner, the PIN and a drawn password, so a
script can capture the diagnostics without its own display.

### Values and conflicts

- A command line that says two contradictory things exits 2 rather than choosing: the identity
  comes from exactly one of `--seed`, `--seed-file`, `-key <file>` and `-key`;
  `-h265` and `--features` cannot both set the features; the password is set by one of
  `-pw`, `--password`, `--random-password` and `--password-file`; `-p <base>` cannot be
  combined with `--port` or with a single media port.
- An option that takes a value refuses to take the next option for it: `-n -p 7000` reports
  that `-n` needs a value instead of naming the receiver `-p`. Numeric options still accept a
  leading dash, so `-db -30:0` is a range.
- Floats are checked for finiteness: NaN and infinity are refused, not clamped.

## 6. Recipes

```sh
# A receiver in the living room, with a fixed PIN
parlor -n "Living room" -pin 1234

# Check the network and a pairing without a sound card
parlor --render null -v

# Lowest latency at 1080p30 with the hardware decoder
parlor -s 1920x1080 -fps 30 -vs d3d11videosink -vd d3d11h264dec

# 4K HEVC, both decoders named
parlor -h265 -s 3840x2160 -vs d3d11videosink -vd d3d11h264dec --video-decoder-hevc d3d11h265dec

# Nothing left behind: no identity file, no client list
parlor -key --ephemeral-seed

# Only this machine: pair with a client that is told 127.0.0.1:7000
parlor --bind 127.0.0.1 --no-advertise

# One device only
parlor -allow aa:bb:cc:dd:ee:ff

# Keep the art and the now-playing text
parlor -ca cover.png -md now-playing.txt

# A password that never appears in a process list
parlor --password-file /run/secrets/parlor
```

## 7. Files

| File | Default location | What it is |
| --- | --- | --- |
| `identity.seed` | `%LOCALAPPDATA%\parlor`, `$XDG_DATA_HOME/parlor` or `$HOME/.local/share/parlor` | 64 hex characters; the identity every key derives from |
| `clients` | beside it | one 64 character hex key per line: clients that paired |

`--data-dir <path>` moves both. `-key` and `-reg` move one each. On Unix the seed is created
readable by its owner alone (0600). A seed that exists and cannot be read stops the run: it is
reported rather than replaced, because a new seed would drop every pairing.

`-key` without a file keeps the identity in memory for the run, and then the client list stays
in memory too unless `-reg` names a file.

## 8. Exit codes

| Code | Meaning |
| --- | --- |
| 0 | the run succeeded; also `--help` and `--version` |
| 1 | a run that could not start or that failed: an unreadable identity, a password file, a missing backend, a port already in use |
| 2 | a command line this program cannot accept: an unknown option, a missing value, a value out of range, two options that contradict each other |

## 9. Troubleshooting

Start with `-v`; it prints the pipeline in use and what the receiver thinks of its stream.

### There is no picture at all

1. `-v` should show `*** mirror stream keyed` and then `video stream open in ... ms`. If the
   first line is missing, the device never reached the mirror port: check the firewall (control
   port 7000 and the mirror TCP port) and that both are on the same network.
2. If it shows `rust-airplay: cannot show video: ...`, the pipeline did not build. Read the
   reason; usually a missing element (`gst-inspect-1.0 d3d11h264dec`) or a sink that cannot sit
   behind that decoder. Fall back with `-vd avdec_h264`.
3. The window is created by the sink when mirroring starts and does not take focus. Look for it
   in the task bar or Alt+Tab.
4. A build without the `gst` feature refuses `--render gst` with exit 1 and says so.

### The picture stops but the device keeps sending

This was a defect in the 0.3.1 receiver: after one queue overflow it waited for a key frame
that a still screen never sends. It is fixed in 0.3.2, which this release uses. If you see
`written` stop climbing while `received` keeps going on 0.3.2, report it with the log.

### The picture stutters

```text
media     mirror: 308 packets, 3946087 bytes in, 297 received, 297 written, 1 waiting, 0 dropped
```

- `waiting` at 8 with `dropped` climbing: the renderer cannot keep up. Use the hardware
  decoder (`-vd d3d11h264dec`), ask for a smaller picture (`-s 1280x720`), or lower `-fps`.
- `queued` in the hundreds of milliseconds in the `pace frame` lines says the same thing.
- A stream that arrives in bursts judders even when nothing is dropped; `--pace-video` spreads
  the frames at the cost of at most 40 ms each.

### There is no audio

`-v` should show `audio stream open, aac-eld at 44100`. If it is missing, the client did not
start audio. If the pipeline cannot open the sink, name one: `-as wasapisink` on Windows,
`-as pulsesink` or `-as alsasink` on Linux, and check it with `gst-inspect-1.0`.

### The PIN never appears

A fixed PIN (`-pin 1234`) is not printed; a one-time PIN is printed when a client asks for
one. With `--no-pin` no PIN is used at all.

### The port is already in use

The run exits 1 with the port named. Another receiver, or another program, holds it: choose
another with `--port`, or let the system choose with `--port 0` (and then discover the
receiver rather than typing its port).

### `--render gst` says the feature is missing

The binary was built without `--features gst`. Rebuild with it, or use `--render null`.

### gst-launch-1.0 is not found

The video pipeline runs in `gst-launch-1.0`. Put GStreamer's `bin` on `PATH`, or name the
program with `AIRPLAY_GST_LAUNCH`. Audio still works without it; video does not.

## 10. Security

- **The PIN and the password are the gate.** `-allow` and `-block` compare a client's claimed
  device identifier, which a hostile client can claim to be anything.
- **`-ca`, `-md` and `-dacp` accept writes from the network without authentication.** Any host
  on the local network can send the request and cause the file to be written, and the
  now-playing text to be printed. The program bounds what that can do - control characters are
  stripped, terminal output is cut at 2 KiB, cover art over 4 MiB and metadata over 256 KiB are
  dropped, writes go through a bounded queue - but point those paths somewhere that does not
  matter, and treat the files as public.
- **Keep credentials out of the command line.** `--password` and `--seed` are visible in a
  process list and in shell history; use `--password-file -` (standard input) and
  `--seed-file` instead.
- **The seed is the receiver's identity.** Anyone who has `identity.seed` can impersonate this
  receiver. It is created 0600 on Unix; on Windows it relies on the ACL of your profile
  directory.
- The protocol itself is unencrypted RTSP with an MD5 digest; it is designed for a home
  network.
