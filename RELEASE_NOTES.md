# parlor 0.3.0

`parlor` is an AirPlay 1 and AirPlay 2 receiver for Windows and Linux, run from a terminal. It
advertises a receiver on the local network, pairs with an iPhone, iPad or Mac using a PIN or a
password, and plays screen mirroring and audio through GStreamer. The protocol lives in the
[`rust-airplay`](https://crates.io/crates/rust-airplay) library and playback in
[`rust-airplay-gstream`](https://crates.io/crates/rust-airplay-gstream); `parlor` is the
control panel around them.

0.3.0 is about the command line. It can print its version, listen on one address, keep its
files where you say, read a password from a file or standard input, and stay off mDNS when you
do not want it announced. A command line that contradicts itself is an error instead of a
silent choice, the exit code says whether the command line or the receiver was at fault, and
the files a client can make it write are bounded and written down.

## What's new

- `--version` and `-V`, so a script can check what it is running.
- `--bind <addr>` to listen on one address instead of every interface, and `--no-advertise` to
  keep the receiver off mDNS.
- `--password-file <path>` - or `-` for standard input - and `--seed-file <path>`.
- `--data-dir <path>`, which moves the identity seed and the client list somewhere else.
- Exit codes: 0 for success, 1 for a run that could not start or that failed, 2 for a command
  line the program cannot accept.
- Contradictory options are refused with exit code 2: two identity sources, `-h265` with
  `--features`, two password sources, and `-p <base>` with an individual port.
- `-allow` and `-block` normalise device identifiers to twelve hex digits, so a colon- or
  dash-separated address names the same device as a bare one; a malformed value is an error.
  `-allow` on its own now turns on `-restrict`, with a warning.
- `-ca` without a path reports each image's type and byte count instead of doing nothing.
- `-h265` offers 4K only when no size was given, so `-h265 -s 1920x1080` stays at 1080p.
- The log goes to standard error; standard output keeps the banner, the PIN and a drawn
  password.
- 33 unit tests and seven integration tests, none of which need GStreamer.

## Upgrading from 0.2.0

Two changes are breaking. The dependencies move to the published 0.3 releases of both
libraries, so a lock file pinned to the 0.2 line has to move with them. And a command line the
program cannot accept now exits 2 instead of 1, so a script that treated any non-zero exit as
`the receiver failed to start` should branch on 2 first.

[`MIGRATION.md`](MIGRATION.md) has the rest: the option combinations that are now refused, the
`-allow` change, the stricter numeric checks, and what to do if the identity file is damaged -
0.3.0 stops rather than quietly generating a new identity, because that would drop every
pairing. The default data directory and both file formats are unchanged.

## Getting started

```sh
parlor -n "Living room" -pin 1234
```

`--help` lists every option. The README has the full option table, the exit codes, the files
`parlor` keeps, and the limitations that come with `-ca`, `-md` and `-dacp`.

## Downloads

- Linux: `parlor-v0.3.0-x86_64-unknown-linux-gnu.tar.gz`
- Windows: `parlor-v0.3.0-x86_64-pc-windows-gnu.zip`

Extract the archive and run `parlor` (or `parlor.exe`) from the directory it creates.

## Notes

- A one-time PIN is printed when it is drawn. `-pw` without a password draws one for the
  process and prints it.
- A build from source without the `gst` feature still reads a session, but plays nothing.
- With `-ca`, `-md` or `-dacp`, an unauthenticated host on the local network can ask for a
  write or a line of terminal output. 0.3.0 bounds it; a real fix needs authentication in the
  library. See the README's known limitations.
- The archives are built with the GStreamer backend, so the machine that runs one needs
  GStreamer 1.0 and its base plugins.
- GPL-3.0-or-later. The archives include the licence; the source is on GitHub.
