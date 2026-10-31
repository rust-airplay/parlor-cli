# Changelog

## 0.3.0 - 2026-10-30

The command line is audited, the program is split into a library and a thin entry point, and
both dependencies move to 0.3. Two changes are breaking: the dependency line, and the exit code
an unacceptable command line earns.

### Added

- `--version` and `-V` print the version and exit successfully.
- `--bind <addr>` names the address to listen on, IPv4 or IPv6; the default is still every
  interface.
- `--no-advertise` starts a receiver that is not announced over mDNS.
- `--password-file <path|->` reads the SETUP password from a file, or from standard input for
  `-`; `--seed-file <path>` reads the identity seed from a file.
- `--data-dir <path>` names the directory the identity and the client list live in.
- `--video-decoder-hevc <decoder>` names the decoder for an H.265 stream. `-vd` names the
  H.264 one, and since the 0.3 backend a sink no longer brings a decoder with it.
- `--pin` and `--quiet` are in the usage text. Both worked; neither was listed.
- Exit codes: 0 for a successful run, 1 for a run that could not start or that failed, 2 for a
  command line this program cannot accept.
- The program is a library: `[lib] name = "parlor"`, with the command line, the plan, the
  files, the console and the renderer in `cli`, `plan`, `files`, `console` and `render`.
  `src/main.rs` is the entry point only.
- 33 unit tests, and seven integration tests in `tests/cli.rs` that run the built binary
  through `CARGO_BIN_EXE_parlor`.

### Changed

- **Breaking.** Both dependencies move from the 0.2 line to the published 0.3 releases:
  `rust-airplay` 0.3 and `rust-airplay-gstream` 0.3. A build pinned to 0.2 has to move with
  them.
- **Breaking.** A command line this program cannot accept exits 2, where it used to exit 1, so
  a script can tell `fix the command line` from `look at the receiver`.
- Contradictory options are errors rather than a silent choice: the identity is one of
  `--seed`, `--seed-file`, `-key <file>` and `-key`/`--ephemeral-seed`; `-h265` and
  `--features` cannot both set the advertised features; `-pw`, `--password` and
  `--random-password` cannot be combined with `--password-file`; `-p <base>` cannot be
  combined with `--port` or with a single media port.
- Device identifiers given to `-allow` and `-block` are normalised to twelve upper-case
  hexadecimal digits, so a colon- or dash-separated address names the same device as a bare
  one, and anything else is refused instead of matching nothing.
- `-allow` without `-restrict` turns `-restrict` on and prints a warning, because the library
  would otherwise ignore the list.
- `-ca` without a path reports the cover art's content type and byte count on standard error
  instead of doing nothing.
- `-h265` enlarges the default display to 4K only when no size was stated;
  `-h265 -s 1920x1080` keeps 1080p.
- All log output goes to standard error. Standard output keeps the banner, the PIN and a drawn
  password.
- The identity seed is created readable by its owner alone (0600) on Unix, and only when it
  does not exist: a seed that exists and cannot be read is an error, not a new identity.
- A client is appended to the client list only when it is new, so a client that reconnects
  does not grow the file. `-key` without a file keeps the registry in memory unless `-reg`
  names a file.
- An option that takes a value no longer swallows the option after it: `-n -p 7000`,
  `-allow -restrict` and `-md -q` report that a value is needed. Numeric options still accept
  a value that begins with a dash, so `-db -30:0` is a range.

### Fixed

- NaN and infinity are refused by `-vol`, `-al`, `--volume`, `--audio-latency` and `-reset`.
  They used to pass a range check and become a value nobody asked for.
- `-s` is bounded like the other size options: at most 16384 pixels each way and a refresh
  rate of 1 to 240. It used to accept any size at all.
- `-reset` is bounded at 86400 seconds, so a value too large for a `Duration` is refused
  instead of reaching it.
- A seed that is not hexadecimal - including one whose bytes are not ASCII - is reported
  instead of panicking on a slice boundary.
- A closed standard output no longer stops the program: every write is allowed to fail.
- A mirror is paced by this program by default (`--pace-video`), instead of being left to the
  backend. Pacing left to the backend meant the library reported every frame's arrival
  interval and the renderer slept it before writing, so the renderer ran a fraction of a frame
  behind the stream, its eight frame queue never drained - about a quarter of a second of
  latency on a 30 fps mirror - and a frame was dropped whenever the two drifted, breaking the
  H.264 reference chain that the rest of the stream depends on. Measured on one machine: the
  queue went from a pinned 8 frames and 360 ms to 1 frame and 123 ms, and the picture stopped
  stuttering. `--no-pace-video` restores the old behaviour and says what it costs.
- A GStreamer run without `-vd` now says at startup that video is decoded in software. In 0.2
  a hardware sink brought its decoder with it; in 0.3 the two are separate choices, which left
  a mirror stream software decoded with a full frame queue - the stutter this release was
  asked about. The README's latency section names the flags that restore the hardware path.

### Performance

- Everything a client can cause to be written goes through one writer thread with a bounded
  queue of 16, so a disk write no longer runs on the async runtime. A full queue drops the
  write and says so instead of holding up the session.

### Security

- Client-controlled now-playing text is stripped of control characters, kept to one line and
  cut at 2 KiB before it reaches the console.
- Cover art over 4 MiB, metadata over 256 KiB and a remote-control export over 4 KiB are
  dropped rather than written.
- The identity seed is created with 0600 permissions on Unix, and an unreadable one stops the
  run instead of being replaced.
- A 64 character hexadecimal token in a log record - an identity seed or a client key - is
  replaced with `<redacted>` before it reaches the console. The library reports its identity at
  the info level, and one verbose session would otherwise hand over the secret a receiver is
  recognised by.
- Known limitation, documented in the README: with `-ca`, `-md` or `-dacp`, a `SET_PARAMETER`
  request from an unauthenticated host on the local network can still cause a file to be
  written and a line to be printed. The bounds above limit what it can do; authenticating the
  request needs a change in the library.

### Runtime notes

- A build with the `gst` feature needs GStreamer 1.0 and its base plugins on the machine that
  runs it; without the feature it needs no media library.
- The migration steps from 0.2.0 are in `MIGRATION.md`.

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
