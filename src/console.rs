// SPDX-License-Identifier: GPL-3.0-or-later
//! The console: one banner, and a log that does not take the receiver down with it.
//!
//! A receiver is usually started from a terminal, a service file or a script with the
//! output piped somewhere. When the reader goes away, writing must not be the thing that
//! stops a session, so every write here is allowed to fail.

use std::io::Write;

/// The longest piece of client-controlled text printed to the terminal.
///
/// A metadata listing is a few lines; a receiver that prints an unbounded listing is a
/// receiver a client can use to flood its own console.
pub const MAX_CONSOLE_TEXT: usize = 2_048;

/// Writes a line to standard output, ignoring a reader that has gone away.
pub fn out(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{text}");
}

/// Writes a formatted line to standard output, ignoring a closed pipe.
pub fn out_line(arguments: std::fmt::Arguments<'_>) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{arguments}");
}

/// Writes a formatted line to standard error, ignoring a closed pipe.
pub fn err_line(arguments: std::fmt::Arguments<'_>) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{arguments}");
}

/// Turns client-controlled text into one line a terminal can show.
///
/// Control characters are dropped rather than escaped: a metadata listing arrives as text
/// and a receiver printing an escape sequence a client sent is a receiver a client can
/// drive. Newlines become spaces so the listing stays one line, and the text is truncated
/// so no single request can flood a console.
#[must_use]
pub fn printable(text: &str) -> String {
    let mut shortened = String::with_capacity(MAX_CONSOLE_TEXT.min(text.len()));
    for character in text.chars() {
        let character = if character == '\n' { ' ' } else { character };
        if character.is_control() {
            continue;
        }
        if shortened.len() + character.len_utf8() > MAX_CONSOLE_TEXT {
            break;
        }
        shortened.push(character);
    }
    shortened
}

/// Prints the banner a receiver is introduced by.
pub fn banner(name: &str, port: &str, playback: &str, clients: usize) {
    out_line(format_args!(
        "parlor {} - an AirPlay receiver\n",
        crate::VERSION
    ));
    out_line(format_args!("  Name      {name}"));
    out_line(format_args!("  Port      {port}"));
    out_line(format_args!("  Playback  {playback}"));
    out_line(format_args!("  Clients   {clients} remembered\n"));
    out_line(format_args!(
        "Waiting for a client. Press Ctrl+C to stop.\n"
    ));
}

/// The log: every record goes to standard error.
///
/// Standard output carries what a person reads (the banner, a PIN, a drawn password), so a
/// script can capture the session's diagnostics separately; and a fatal error and a
/// warning then leave through the same door, which is what a supervisor expects.
struct Terminal;

impl log::Log for Terminal {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let message = record.args().to_string();
        match redact(&message) {
            Some(redacted) => err_line(format_args!("{redacted}")),
            None => err_line(format_args!("{message}")),
        }
    }

    fn flush(&self) {}
}

/// The length of a hex string that is a seed or a client key.
const SECRET_HEX: usize = 64;

/// Replaces a 64 character hexadecimal token in a log record, if there is one.
///
/// The library reports its identity at the info level, and an identity seed is the secret
/// a receiver is recognised by: one screenshot of a verbose session would hand it over.
/// The same shape is a remembered client key, which is worth no more on a console. A
/// record that holds neither is returned unchanged, and nothing is allocated for it.
#[must_use]
pub fn redact(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut start = 0;
    while start + SECRET_HEX <= bytes.len() {
        let candidate = &bytes[start..start + SECRET_HEX];
        let hex = candidate.iter().all(u8::is_ascii_hexdigit);
        let bounded = (start == 0 || !bytes[start - 1].is_ascii_hexdigit())
            && (start + SECRET_HEX == bytes.len()
                || !bytes[start + SECRET_HEX].is_ascii_hexdigit());
        if hex && bounded {
            let mut redacted = String::with_capacity(text.len());
            redacted.push_str(&text[..start]);
            redacted.push_str("<redacted>");
            redacted.push_str(&text[start + SECRET_HEX..]);
            return Some(redacted);
        }
        start += 1;
    }
    None
}

static TERMINAL: Terminal = Terminal;

/// Installs the console log at a level.
pub fn install(level: log::LevelFilter) {
    if log::set_logger(&TERMINAL).is_ok() {
        log::set_max_level(level);
    }
}

#[cfg(test)]
mod tests {
    use super::{printable, redact};

    #[test]
    fn an_identity_is_not_printed() {
        // The library reports "identity  <64 hex>" at the info level; that is the secret a
        // receiver is recognised by.
        let record = format!("identity  {}", "fc70".repeat(16));
        let redacted = redact(&record).expect("redacted");
        assert!(!redacted.contains("fc70fc70"), "{redacted}");
        assert!(redacted.contains("<redacted>"), "{redacted}");
    }

    #[test]
    fn an_ordinary_record_is_left_alone() {
        assert!(redact("receiver  parlor (b6:c7:d3:6d:c4:4e)").is_none());
        // A hex run that is longer than a key is part of a longer token, not a key.
        assert!(redact(&"a".repeat(65)).is_none());
    }

    #[test]
    fn a_client_cannot_drive_the_terminal() {
        // A listing with a clear-screen sequence and newlines becomes one printable line.
        let text = printable("title\u{1b}[2J\nartist\u{7}");
        assert!(!text.contains('\u{1b}'), "{text:?}");
        assert!(!text.contains('\u{7}'), "{text:?}");
        assert!(!text.contains('\n'), "{text:?}");
    }

    #[test]
    fn a_long_listing_is_truncated() {
        let text = printable(&"a".repeat(super::MAX_CONSOLE_TEXT * 2));
        assert_eq!(text.len(), super::MAX_CONSOLE_TEXT);
    }
}
