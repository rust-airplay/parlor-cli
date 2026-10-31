# parlor 0.3.1

`parlor` is an AirPlay 1 and AirPlay 2 receiver for Windows and Linux, run from a terminal. It
advertises a receiver on the local network, pairs with an iPhone, iPad or Mac using a PIN or a
password, and plays screen mirroring and audio through GStreamer. The protocol lives in the
[`rust-airplay`](https://crates.io/crates/rust-airplay) library and playback in
[`rust-airplay-gstream`](https://crates.io/crates/rust-airplay-gstream); `parlor` is the
control panel around them.

0.3.1 is a short release with one point: it moves to the 0.3.1 libraries, which fix the defect
behind a stutter that no amount of configuration could remove. In 0.3.0 the receiver told the
renderer how far apart frames were arriving and the renderer slept that long before writing
each one. The renderer was therefore always a fraction of a frame behind the stream, its eight
frame queue never emptied, and any jitter cost a frame - which, in an inter-frame coded stream,
damages the picture until the next key frame.

With 0.3.1 the frames go to the renderer as fast as it takes them, and a queue that does fill
is cleared and waits for a key frame instead of corrupting the picture. Measured on one
machine with the same client: the queue went from a pinned eight frames and 360 ms to one frame
and 123 ms, and the stutter stopped.

## Upgrading

If you run 0.3.0, upgrade. The libraries are the fix; `parlor` itself gains only the version.

- The model is still `0.3.1`, and `--version` prints it.
- The picture is no longer paced by `parlor` by default. `--pace-video` remains, for a stream
  that arrives in bursts; it spends at most 40 ms a frame to smooth them.
- A command line that worked with 0.3.0 keeps working.

## Also in this release

- A **Requirements** section in the README: Rust 1.85, GStreamer 1.0 for the `gst` feature, and
  0.3.1 or newer of both libraries.
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

- Linux: `parlor-v0.3.1-x86_64-unknown-linux-gnu.tar.gz`
- Windows: `parlor-v0.3.1-x86_64-pc-windows-gnu.zip`

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
