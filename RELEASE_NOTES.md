# parlor 0.3.2

`parlor` is an AirPlay 1 and AirPlay 2 receiver for Windows and Linux, run from a terminal. It
advertises a receiver on the local network, pairs with an iPhone, iPad or Mac using a PIN or a
password, and plays screen mirroring and audio through GStreamer. The protocol lives in the
[`rust-airplay`](https://crates.io/crates/rust-airplay) library and playback in
[`rust-airplay-gstream`](https://crates.io/crates/rust-airplay-gstream); `parlor` is the
control panel around them.

0.3.2 is two fixes and a pin.

The pin: the two libraries are now git dependencies, fixed to the revisions this program was
tested against, because the releases it needs are ahead of crates.io. `Cargo.toml` names a
commit for each; the first build clones them once.

The fixes came in two steps, and both were found by running the receiver against a real
iPhone rather than by reading code:

- **0.3.1, the stutter.** In 0.3.0 the receiver told the renderer how far apart frames were
  arriving, and the renderer slept that long before writing each one. It was therefore always
  a fraction of a frame behind the stream, its eight frame queue never emptied, and any jitter
  cost a frame. Measured on one machine: a pinned queue of eight frames and 360 ms of latency
  became one frame and 123 ms.
- **0.3.2, the freeze.** 0.3.1 fixed the stutter by clearing an overflowed queue and waiting
  for a key frame - but an AirPlay client only sends one when its picture changes, so a mostly
  still screen never sent another. A measured session wrote 208 frames and then dropped 240,
  leaving a still picture that no longer followed the device. The wait is now bounded at one
  second, and the startup allowance is large enough that opening a pipeline is not mistaken
  for an overflow.

## Upgrading

If you run 0.3.0 or 0.3.1, upgrade. The libraries are the fix; `parlor` itself gains the pin
and the version.

- `--version` prints `0.3.2`.
- A lock file from an older release has to be regenerated: `cargo update`.
- The picture is no longer paced by `parlor` by default. `--pace-video` remains, for a stream
  that arrives in bursts; it spends at most 40 ms a frame to smooth them.
- A command line that worked with 0.3.0 keeps working.

## Also in this release

- A **Requirements** section in the README: Rust 1.85, GStreamer 1.0 for the `gst` feature, and
  what the git dependencies are for.
- A **Diagnosing a mirror** section: the four lines `-v` prints, and what each answer means -
  a healthy stream, a renderer that cannot keep up, a decoder that is not the hardware one.

The command-line work of 0.3.0 still stands: every option in one table, contradictory options
refused instead of silently resolved, exit code 2 for a command line the program cannot accept,
files a client can make it write bounded and documented. See [CHANGELOG.md](CHANGELOG.md) for
the whole list.

## Getting started

```sh
parlor -n "Living room" -pin 1234
```

`--help` lists every option. The README has the option table, the exit codes, the files
`parlor` keeps, how to diagnose a mirror, and the limitations that come with `-ca`, `-md` and
`-dacp`.

## Downloads

- Linux: `parlor-v0.3.2-x86_64-unknown-linux-gnu.tar.gz`
- Windows: `parlor-v0.3.2-x86_64-pc-windows-gnu.zip`

Extract the archive and run `parlor` (or `parlor.exe`) from the directory it creates.

## Notes

- A one-time PIN is printed when it is drawn. `-pw` without a password draws one for the
  process and prints it.
- A build from source without the `gst` feature still reads a session, but plays nothing.
- With `-ca`, `-md` or `-dacp`, an unauthenticated host on the local network can ask for a
  write or a line of terminal output. The program bounds it; a real fix needs authentication in
  the library. See the README's known limitations.
- The archives are built with the GStreamer backend, so the machine that runs one needs
  GStreamer 1.0 and its base plugins.
- GPL-3.0-or-later. The archives include the licence; the source is on GitHub.
