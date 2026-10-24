# Changelog

## 0.2.0 - 2026-10-24

The first release of `parlor`, the command-line AirPlay receiver.

### Receiver

- Advertises one receiver over mDNS, with the name a client shows and the hardware address it
  records.
- Pairing with a one-time PIN, a fixed PIN, or a password. A one-time PIN is printed when it is
  drawn, whatever the log level is.
- The identity seed and the verified clients are kept between runs in `identity.seed` and
  `clients`; `-key` and `-reg` move them, and `-key` without a file keeps the identity in
  memory only so nothing is stored.
- Client policy with `-restrict`, `-allow` and `-block`; `-nohold` lets a new client take over
  from the current one; `-reset` closes a session whose client stopped sending `/feedback`.

### Media

- Screen mirroring and audio through the playback backend, with a choice of GStreamer sinks,
  parser and decoder: `-as`, `-vs`, `-vp`, `-vd` and `-avdec`.
- A build without the `gst` feature still reads and reports a session, through a renderer that
  plays nothing; `--render null` asks for that explicitly.
- The default sinks are the first one the machine actually has, so a Windows build reaches the
  hardware decoder and sink without being told to.
- `-h265` allows HEVC and offers a 4K display; `-s`, `-fps` and `-o` set the display a client
  sees.
- `--warm-video` starts the video pipeline before the first client, and `--pace-video` holds
  mirror frames to their capture spacing.

### Volume

- The volume a client sets is converted from decibels to linear gain and applied to the stream
  that is playing.
- `-vol` sets the starting volume, `-db` rescales the slider onto a decibel range, and `-taper`
  selects the tapered profile instead of the flat one.

### Command line

- Short options are the conventional receiver names, with long forms for scripts. The list is in
  `--help` and the README.
- Values are checked where the protocol or the backend uses them: PINs are 0001 to 9999,
  passwords need at least four characters, the volume is 0 to 1, the audio delay 0 to 10
  seconds, a decibel range runs low to high, a display has pixels, and a port layout stays
  inside the port range.
- An option the receiver cannot honour is refused with the reason instead of being ignored.
- `-p n` lays out the ports as `n..n+2` with the control port at `n+1`; `-p` alone selects the
  fixed layout.

### Console

- One banner, then warnings and errors by default. `-q`, `-v`, `-d` and `--log-level` choose
  what the console shows, and a PIN or a drawn password is printed regardless.
- `--log <file>` writes the request trace to a file, independently of the console level.

### Documentation and tests

- `README.md` and `README.zh-CN.md`, in English and Chinese.
- Eight command-line tests cover the option names, the port layout, the optional values and the
  refusal of values outside their range. They need no GStreamer.
- CI runs the tests on Ubuntu and Windows; a manual release workflow builds `parlor` with the
  GStreamer backend for both systems and creates a GitHub release with the notes it is given.

### Runtime notes

- A build with the `gst` feature needs GStreamer 1.0 and its base plugins on the machine that
  runs it; without the feature it needs no media library.
- `-pw` without a password draws one for the process and prints it; it does not change per
  connection.
