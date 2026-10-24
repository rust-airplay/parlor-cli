# parlor 0.2.0

The first release of `parlor`, an AirPlay 1 and AirPlay 2 receiver for Windows and Linux.

`parlor` runs in a terminal: it advertises a receiver on the local network, pairs with an iPhone
or iPad using a PIN or password, and plays screen mirroring and audio through GStreamer. The
protocol lives in the [`rust-airplay`](https://crates.io/crates/rust-airplay) library and
playback in [`rust-airplay-gstream`](https://crates.io/crates/rust-airplay-gstream); `parlor` is
the control panel around them.

## What you can do

- Pick the receiver from the AirPlay menu of an iPhone, iPad or Mac, and mirror the screen.
- Send audio from a client and change its volume; the client's decibels become a linear gain on
  the stream that is playing.
- Choose the GStreamer sinks, parser and decoder, or use the hardware path the machine already
  has.
- Pair with a one-time PIN, a fixed PIN or a password, and keep the receiver recognisable across
  restarts.
- Run with `--render null` on a machine with no sound card or display to inspect a session.

## Getting started

```sh
parlor -n "Living room" -pin 1234
```

`--help` lists every option. The README has the full option table, the files `parlor` keeps and
the port layout.

## Downloads

- Linux: `parlor-v0.2.0-x86_64-unknown-linux-gnu.tar.gz`
- Windows: `parlor-v0.2.0-x86_64-pc-windows-gnu.zip`

Extract the archive and run `parlor` (or `parlor.exe`) from the directory it creates.

## Notes

- This is the first release; the command line and the file formats are the ones this version
  documents.
- A one-time PIN is printed when it is drawn. `-pw` without a password draws one for the process
  and prints it.
- A build from source without the `gst` feature still reads a session, but plays nothing.
- GPL-3.0-or-later. The archives include the licence; the source is on GitHub.
