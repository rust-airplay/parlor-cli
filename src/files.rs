// SPDX-License-Identifier: GPL-3.0-or-later
//! The files a run keeps, and the writer that keeps disk work off the async runtime.
//!
//! Two files matter: the identity seed, which is the secret a receiver is recognised by,
//! and the registry of clients whose keys a previous run verified. Both live under one
//! directory, both are written atomically, and on a system with Unix permissions the seed
//! is created readable by its owner alone.
//!
//! Everything a client can cause to be written goes through [Outputs]: a bounded channel
//! drained by one thread. The callbacks the library calls are synchronous and are called
//! from the async runtime, so doing the write there would block a worker on a disk; a
//! bounded queue also means a client that sends artwork faster than it can be written is
//! dropped rather than allowed to grow the process without limit.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};

use rust_airplay::receiver::ClientCallback;

use crate::Error;
use crate::cli::{ClientsSource, CoverArt, Identity, Options, Password};
use crate::console;

/// The largest piece of cover art that is written.
pub const MAX_ARTWORK: usize = 4 * 1024 * 1024;

/// The largest metadata listing that is written.
pub const MAX_METADATA: usize = 256 * 1024;

/// The largest remote-control export that is written.
pub const MAX_DACP: usize = 4 * 1024;

/// How many writes may be queued before a new one is dropped.
const QUEUE: usize = 16;

/// Reads a 32 byte seed from hexadecimal.
///
/// The text is read as bytes, not as characters: a 64 byte string whose bytes are not
/// characters at all is a corrupt file or a mistyped argument, and reporting that is the
/// point. Slicing a string at two-byte boundaries would panic on the second byte of a
/// multi-byte character instead, which is a crash where a message belongs.
///
/// # Errors
///
/// Returns a message naming what was wrong: the length, a non-hexadecimal pair, or bytes
/// that are not ASCII.
pub fn parse_hex32(text: &str) -> Result<[u8; 32], String> {
    let bytes = text.trim().as_bytes();
    if bytes.len() != 64 {
        return Err(format!(
            "expected 64 hex characters (32 bytes), got {}",
            bytes.len()
        ));
    }
    let mut seed = [0u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        let pair = &bytes[index * 2..index * 2 + 2];
        let pair = std::str::from_utf8(pair)
            .map_err(|_| "a seed is hexadecimal, so it is ASCII".to_owned())?;
        *byte = u8::from_str_radix(pair, 16).map_err(|_| format!("{pair:?} is not hexadecimal"))?;
    }
    Ok(seed)
}

/// Writes bytes as lower-case hexadecimal.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The directory this program keeps its files in.
///
/// # Errors
///
/// Returns a message when the caller named no directory and the platform has none either.
pub fn data_directory(options: &Options) -> Result<PathBuf, Error> {
    if let Some(path) = &options.data_dir {
        return Ok(path.clone());
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/share")))
        .ok_or_else(|| {
            Error::run(
                "neither LOCALAPPDATA, XDG_DATA_HOME nor HOME is set; name one with --data-dir",
            )
        })?;
    Ok(base.join("parlor"))
}

/// Where the identity seed lives, when the caller did not name a file.
fn seed_path(options: &Options) -> Result<PathBuf, Error> {
    Ok(data_directory(options)?.join("identity.seed"))
}

/// Where the remembered-client registry lives, or nothing when it is kept in memory.
///
/// # Errors
///
/// Returns a message when a default path is needed and no data directory can be found.
pub fn clients_path(options: &Options) -> Result<Option<PathBuf>, Error> {
    Ok(match &options.clients {
        ClientsSource::Memory => None,
        ClientsSource::File(path) => Some(path.clone()),
        ClientsSource::Default => Some(data_directory(options)?.join("clients")),
    })
}

/// The identity seed for this run.
///
/// # Errors
///
/// Returns a message when a named seed file cannot be read, holds something that is not a
/// seed, or cannot be created.
pub fn identity(options: &Options) -> Result<[u8; 32], Error> {
    match &options.identity {
        Identity::Given(seed) => Ok(*seed),
        Identity::Ephemeral => {
            log::info!("identity  ephemeral (nothing is stored)");
            random_seed()
        }
        Identity::File(path) => load_or_create_seed(path),
        Identity::Default => load_or_create_seed(&seed_path(options)?),
    }
}

/// Reads a seed, creating one on first run and never replacing an unreadable one.
///
/// Only a file that does not exist is replaced by a new identity: a file that exists and
/// cannot be read is a permission problem, a path that is a directory, or a seed somebody
/// edited, and generating a new one would silently drop every pairing the old one had.
fn load_or_create_seed(path: &Path) -> Result<[u8; 32], Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            parse_hex32(&text).map_err(|error| Error::run(format!("{}: {error}", path.display())))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let seed = random_seed()?;
            write_atomic(path, hex(&seed).as_bytes())
                .map_err(|error| Error::run(format!("cannot write {}: {error}", path.display())))?;
            log::info!("identity  created at {}", path.display());
            Ok(seed)
        }
        Err(error) => Err(Error::run(format!(
            "cannot read {}: {error}",
            path.display()
        ))),
    }
}

/// A random seed from the same source the pairing service uses.
fn random_seed() -> Result<[u8; 32], Error> {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed)
        .map_err(|error| Error::run(format!("no random bytes: {error}")))?;
    Ok(seed)
}

/// The password this run gates SETUP with, when it has one.
///
/// # Errors
///
/// Returns a message when a password file cannot be read or holds something too short.
pub fn password(options: &Options) -> Result<Option<String>, Error> {
    match &options.password {
        Password::None => Ok(None),
        Password::Fixed(password) => Ok(Some(password.clone())),
        Password::Random => {
            let mut bytes = [0u8; 6];
            getrandom::getrandom(&mut bytes)
                .map_err(|error| Error::run(format!("no random bytes: {error}")))?;
            Ok(Some(hex(&bytes)))
        }
        Password::FromFile(path) if path == Path::new("-") => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .map_err(|error| {
                    Error::run(format!("cannot read the password from stdin: {error}"))
                })?;
            check_password(&text, "stdin")
        }
        Password::FromFile(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| Error::run(format!("cannot read {}: {error}", path.display())))?;
            check_password(&text, &path.display().to_string())
        }
    }
}

/// Trims a password that was stored with its line ending, and checks its length.
fn check_password(text: &str, origin: &str) -> Result<Option<String>, Error> {
    let password = text.trim_end_matches(['\n', '\r']);
    if password.chars().count() < 4 {
        return Err(Error::run(format!(
            "{origin}: a password needs at least four characters"
        )));
    }
    Ok(Some(password.to_owned()))
}

/// Reads the remembered client keys, one 64 character hex key per line.
///
/// A line that is not a key is skipped rather than failing the receiver: a file somebody
/// edited by hand must not stop a receiver from starting. The count is logged, because a
/// registry that quietly lost half its clients is a thing the user has to know.
#[must_use]
pub fn load_clients(path: &Path) -> HashSet<[u8; 32]> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    let mut clients = HashSet::new();
    let mut skipped = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_hex32(line) {
            Ok(key) => {
                clients.insert(key);
            }
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        log::warn!(
            "{}: {skipped} line(s) were not client keys and were skipped",
            path.display()
        );
    }
    clients
}

/// Keeps the remembered-client registry and the set the library reads in step.
pub struct ClientRegistry {
    path: Option<PathBuf>,
    clients: Arc<Mutex<HashSet<[u8; 32]>>>,
}

impl ClientRegistry {
    /// Wraps the set a run shares with the library.
    #[must_use]
    pub fn new(path: Option<PathBuf>, clients: Arc<Mutex<HashSet<[u8; 32]>>>) -> Self {
        Self { path, clients }
    }

    /// The callback the library calls when it verifies a client for the first time.
    ///
    /// Nothing is returned when the registry is kept in memory: the run then remembers the
    /// client for as long as it lasts, and leaves no file behind.
    #[must_use]
    pub fn callback(&self) -> Option<ClientCallback> {
        let path = self.path.clone()?;
        let clients = Arc::clone(&self.clients);
        Some(Arc::new(move |client: &[u8; 32]| {
            // A client that pairs again is already remembered; appending it twice would
            // grow the file on every reconnection.
            let fresh = clients
                .lock()
                .map(|mut set| set.insert(*client))
                .unwrap_or(false);
            if !fresh {
                return;
            }
            if let Some(parent) = path.parent() {
                if let Err(error) = std::fs::create_dir_all(parent) {
                    console::err_line(format_args!(
                        "parlor: cannot create {}: {error}",
                        parent.display()
                    ));
                    return;
                }
            }
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                Ok(mut file) => {
                    let line = hex(client);
                    if let Err(error) = writeln!(file, "{line}") {
                        console::err_line(format_args!(
                            "parlor: cannot write {}: {error}",
                            path.display()
                        ));
                    }
                }
                Err(error) => console::err_line(format_args!(
                    "parlor: cannot open {}: {error}",
                    path.display()
                )),
            }
        }))
    }
}

/// One write a client asked for.
enum Job {
    /// Bytes to replace a file with.
    Bytes {
        /// Where they go.
        path: PathBuf,
        /// The bytes.
        bytes: Vec<u8>,
        /// What they are, for a message.
        what: &'static str,
    },
    /// Text to replace a file with.
    Text {
        /// Where it goes.
        path: PathBuf,
        /// The text.
        text: String,
        /// What it is, for a message.
        what: &'static str,
    },
}

/// The writes a run performs for a client, and the thread that performs them.
pub struct Outputs {
    cover_art: Option<CoverArt>,
    metadata: Option<PathBuf>,
    dacp: Option<PathBuf>,
    sender: Option<SyncSender<Job>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Outputs {
    /// Sets up the writes a command line asked for.
    ///
    /// # Errors
    ///
    /// Returns a message when a default path is needed and no data directory can be found.
    pub fn new(options: &Options) -> Result<Self, Error> {
        let dacp = match &options.dacp {
            Some(Some(path)) => Some(path.clone()),
            Some(None) => Some(data_directory(options)?.join("dacp")),
            None => None,
        };
        // Only a path is a file: -ca without one reports the art and writes nothing.
        let mut written: Vec<String> = Vec::new();
        if let Some(CoverArt::File(path)) = &options.cover_art {
            written.push(path.display().to_string());
        }
        if let Some(path) = &options.metadata {
            written.push(path.display().to_string());
        }
        if let Some(path) = &dacp {
            written.push(path.display().to_string());
        }
        let (sender, worker) = if written.is_empty() {
            (None, None)
        } else {
            spawn_writer()
        };
        if !written.is_empty() {
            // A receiver that writes a file on behalf of anyone on the network should say
            // so once, where the person who started it is looking.
            console::err_line(format_args!(
                "parlor: unauthenticated network input can write {} (see Known limitations)",
                written.join(", ")
            ));
        }
        Ok(Self {
            cover_art: options.cover_art.clone(),
            metadata: options.metadata.clone(),
            dacp,
            sender,
            worker,
        })
    }

    /// The callback the library calls with cover art.
    #[must_use]
    pub fn artwork_callback(&self) -> Option<rust_airplay::receiver::ArtworkCallback> {
        let cover_art = self.cover_art.clone()?;
        let sender = self.sender.clone();
        Some(Arc::new(move |mime: &str, bytes: &[u8]| {
            if bytes.len() > MAX_ARTWORK {
                console::err_line(format_args!(
                    "parlor: cover art dropped: {} bytes is more than {MAX_ARTWORK}",
                    bytes.len()
                ));
                return;
            }
            match &cover_art {
                // -ca without a file is "report the art": say what arrived.
                CoverArt::Report => console::err_line(format_args!(
                    "parlor: {} cover art, {} bytes",
                    console::printable(mime),
                    bytes.len()
                )),
                CoverArt::File(path) => submit(
                    &sender,
                    Job::Bytes {
                        path: path.clone(),
                        bytes: bytes.to_vec(),
                        what: "cover art",
                    },
                ),
            }
        }))
    }

    /// The callback the library calls with a now-playing listing.
    #[must_use]
    pub fn metadata_callback(&self) -> Option<rust_airplay::receiver::MetadataCallback> {
        let path = self.metadata.clone()?;
        let sender = self.sender.clone();
        Some(Arc::new(move |bytes: &[u8]| {
            if bytes.len() > MAX_METADATA {
                console::err_line(format_args!(
                    "parlor: metadata dropped: {} bytes is more than {MAX_METADATA}",
                    bytes.len()
                ));
                return;
            }
            let text = rust_airplay::media::dmap::text(bytes);
            console::err_line(format_args!(
                "parlor: metadata {}",
                console::printable(&text)
            ));
            submit(
                &sender,
                Job::Text {
                    path: path.clone(),
                    text,
                    what: "metadata",
                },
            );
        }))
    }

    /// The callback the library calls with a request's remote-control identifiers.
    #[must_use]
    pub fn remote_control_callback(&self) -> Option<rust_airplay::receiver::RemoteControlCallback> {
        let path = self.dacp.clone()?;
        let sender = self.sender.clone();
        Some(Arc::new(move |dacp: &str, remote: &str| {
            let text = format!("{dacp}\n{remote}\n");
            if text.len() > MAX_DACP {
                return;
            }
            submit(
                &sender,
                Job::Text {
                    path: path.clone(),
                    text,
                    what: "remote control",
                },
            );
        }))
    }

    /// Waits for the writes still queued, and stops the thread.
    pub fn finish(&mut self) {
        self.sender = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Outputs {
    fn drop(&mut self) {
        self.finish();
    }
}

/// Starts the writer thread, or reports that there is none to start.
fn spawn_writer() -> (Option<SyncSender<Job>>, Option<std::thread::JoinHandle<()>>) {
    let (sender, receiver) = sync_channel::<Job>(QUEUE);
    match std::thread::Builder::new()
        .name("parlor-files".to_owned())
        .spawn(move || {
            while let Ok(job) = receiver.recv() {
                run_job(job);
            }
        }) {
        Ok(worker) => (Some(sender), Some(worker)),
        Err(error) => {
            // Without a thread the writes still happen, on the caller's; slower, and it
            // blocks a runtime worker, which is worth one line on the console.
            console::err_line(format_args!(
                "parlor: no writer thread ({error}); writes happen inline"
            ));
            (None, None)
        }
    }
}

/// Queues a write, or performs it here when there is no thread to queue it to.
fn submit(sender: &Option<SyncSender<Job>>, job: Job) {
    let Some(sender) = sender else {
        run_job(job);
        return;
    };
    match sender.try_send(job) {
        Ok(()) => {}
        Err(TrySendError::Full(job)) => {
            let (what, bytes) = match &job {
                Job::Bytes { bytes, what, .. } => (*what, bytes.len()),
                Job::Text { text, what, .. } => (*what, text.len()),
            };
            console::err_line(format_args!(
                "parlor: {what} dropped: the writer is behind and {bytes} bytes did not fit"
            ));
        }
        Err(TrySendError::Disconnected(job)) => run_job(job),
    }
}

/// Performs one write.
fn run_job(job: Job) {
    let (path, outcome, what) = match job {
        Job::Bytes { path, bytes, what } => {
            let outcome = write_atomic(&path, &bytes);
            (path, outcome, what)
        }
        Job::Text { path, text, what } => {
            let outcome = write_atomic(&path, text.as_bytes());
            (path, outcome, what)
        }
    };
    match outcome {
        Ok(()) => log::info!("{what} -> {}", path.display()),
        Err(error) => console::err_line(format_args!(
            "parlor: cannot write {}: {error}",
            path.display()
        )),
    }
}

/// Replaces a file with bytes, creating its directory and never leaving half a file.
///
/// A client can send the same listing every second, so a reader that looks while a write
/// is in progress must see the old file or the new one, not half of either.
///
/// # Errors
///
/// Returns the first thing the filesystem refused.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    let name = path.file_name().map_or_else(
        || "parlor.tmp".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let temporary = match parent {
        Some(parent) => parent.join(format!(".{name}.tmp")),
        None => PathBuf::from(format!(".{name}.tmp")),
    };
    {
        let mut file = private_options().open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, path)
}

/// The options a file this program writes is opened with.
///
/// The seed is the secret a receiver is recognised by, so on a system with Unix
/// permissions it is created readable by its owner alone.
fn private_options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

#[cfg(test)]
mod tests {
    use super::{hex, parse_hex32, write_atomic};

    #[test]
    fn a_seed_is_thirty_two_bytes_of_hexadecimal() {
        let seed = parse_hex32(&"2a".repeat(32)).expect("parsed");
        assert_eq!(seed, [0x2a; 32]);
        assert_eq!(hex(&seed), "2a".repeat(32));
    }

    #[test]
    fn a_seed_that_is_not_ascii_is_refused_rather_than_sliced_apart() {
        // Sixty four bytes whose second byte is inside a multi-byte character: slicing the
        // string at a two-byte boundary would panic, and a corrupt file must not crash a
        // receiver.
        let text = format!("a{}{}a", "e".repeat(0), "\u{e9}".repeat(31));
        assert_eq!(text.len(), 64);
        let error = parse_hex32(&text).expect_err("refused");
        assert!(
            error.contains("ASCII") || error.contains("hexadecimal"),
            "{error}"
        );
    }

    #[test]
    fn a_seed_of_the_wrong_length_is_refused() {
        assert!(parse_hex32("2a").is_err());
        assert!(parse_hex32(&"2a".repeat(33)).is_err());
        assert!(parse_hex32(&"zz".repeat(32)).is_err());
    }

    #[test]
    fn a_file_is_replaced_whole() {
        let directory = std::env::temp_dir().join(format!("parlor-test-{}", std::process::id()));
        let path = directory.join("art.bin");
        write_atomic(&path, b"first").expect("written");
        assert_eq!(std::fs::read(&path).expect("read"), b"first");
        write_atomic(&path, b"second").expect("written");
        assert_eq!(std::fs::read(&path).expect("read"), b"second");
        let _ = std::fs::remove_dir_all(&directory);
    }
}
