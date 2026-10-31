// SPDX-License-Identifier: GPL-3.0-or-later
//! parlor: an AirPlay receiver on the command line.
//!
//! Everything that makes a receiver a receiver is in the rust-airplay library: the
//! protocol, discovery, the media session and the renderer boundary. This program is the
//! policy around it: it parses arguments, decides where an identity lives, picks a
//! renderer, and prints. It contains no protocol code.
//!
//! The modules are the seams this policy has:
//!
//! - [cli]: words in, [cli::Options] out, and nothing else.
//! - [plan]: [cli::Options] in, the values the library wants out; pure, so it is tested
//!   without a filesystem or a socket.
//! - [files]: the files an identity lives in, and the writer that keeps disk work off the
//!   async runtime.
//! - [console]: the banner and the log, both of which must survive a closed pipe.
//! - [render]: which backend plays the media.
//!
//! See README.md for what the receiver does. The library's API guide is at
//! <https://github.com/rust-airplay/rust-airplay/blob/main/docs/API.md>.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

pub mod cli;
pub mod console;
pub mod files;
pub mod plan;
pub mod render;

pub use cli::{Options, Parsed, RenderBackend, USAGE};

/// The version this program reports.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Why a run did not happen.
///
/// The two kinds are not decoration: a command line this program cannot accept is a
/// mistake to correct, and a run that failed is a receiver to look at. A script that
/// branches on the exit code only has that distinction if the codes differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The command line cannot be accepted; exits 2.
    Usage(String),
    /// The run could not start, or failed; exits 1.
    Run(String),
}

impl Error {
    /// A command line that cannot be accepted.
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    /// A command line this program cannot accept, followed by the usage text.
    pub fn usage_with_usage(message: impl Into<String>) -> Self {
        Self::Usage(format!("{}\n\n{USAGE}", message.into()))
    }

    /// A run that could not start, or that failed.
    pub fn run(message: impl Into<String>) -> Self {
        Self::Run(message.into())
    }

    /// The exit code this error is worth.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Run(_) => 1,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Run(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

/// Runs the program from the process's own arguments, and says how it went.
pub fn main_entry() -> ExitCode {
    match cli::parse_arguments(std::env::args().skip(1)) {
        Ok(Parsed::Help) => {
            console::out(USAGE);
            ExitCode::SUCCESS
        }
        Ok(Parsed::Version) => {
            console::out(&format!("parlor {VERSION}"));
            ExitCode::SUCCESS
        }
        Ok(Parsed::Run(options)) => {
            console::install(options.log_level);
            for warning in &options.warnings {
                log::warn!("{warning}");
            }
            match start(*options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => report(&error),
            }
        }
        Err(error) => report(&error),
    }
}

/// Prints why a run did not happen, where a caller can see it.
fn report(error: &Error) -> ExitCode {
    console::err_line(format_args!("parlor: {error}"));
    ExitCode::from(error.exit_code())
}

/// Starts a receiver and waits for it to stop.
///
/// # Errors
///
/// Returns the first thing that stops the run: an unreadable identity, a password file
/// that cannot be read, a backend this build does not have, or the receiver itself.
pub fn start(options: Options) -> Result<(), Error> {
    run(options)
}

/// The composition root: everything the receiver needs, assembled once.
#[tokio::main]
async fn run(options: Options) -> Result<(), Error> {
    let plan = plan::Plan::derive(&options)?;

    // The two things this program keeps: an identity that survives a restart, and the
    // clients a previous run verified.
    let seed = files::identity(&options)?;
    let clients_file = files::clients_path(&options)?;
    let clients = Arc::new(Mutex::new(
        clients_file
            .as_deref()
            .map(files::load_clients)
            .unwrap_or_default(),
    ));
    let registry = files::ClientRegistry::new(clients_file, Arc::clone(&clients));
    let on_client = registry.callback();

    let password = files::password(&options)?;
    if let Some(password) = &password {
        console::out(&format!("Password for clients: {password}"));
    }

    // The renderer is chosen before anything is advertised, and the banner comes after it:
    // a backend this build or this machine cannot provide is something to say now, not
    // after announcing that a session will play through it.
    let renderer = render::build(&options)?;
    if options.log_level >= log::LevelFilter::Warn {
        let remembered = clients.lock().map(|set| set.len()).unwrap_or(0);
        let port = if plan.control_port == 0 {
            "chosen by the system".to_owned()
        } else {
            plan.control_port.to_string()
        };
        let playback = match options.render {
            RenderBackend::Null => "none (media is read and played nowhere)",
            RenderBackend::Gst => "GStreamer",
        };
        console::banner(&options.name, &port, playback, remembered);
        if !options.advertise {
            log::warn!("--no-advertise: nothing answers a client that looks for this receiver");
        }
        if plan.control_port == 0 {
            log::info!(
                "port      0: the system chooses, and only a client told the port can reach it"
            );
        }
    }

    if options.pace_video {
        // Worth one line, because it is the one setting that deliberately spends latency.
        log::info!("pacing    on: frames are held to their capture spacing, at most 40 ms each");
    }

    let mut outputs = files::Outputs::new(&options)?;

    let mut config = rust_airplay::Config::new(options.name.clone(), seed, renderer);
    config.port = plan.control_port;
    config.bind = options.bind.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    config.advertise = options.advertise;
    config.device_id.clone_from(&options.device_id);
    config.pin = options.pin;
    config.password = password;
    config.audio_codec = options.audio_codec;
    config.clients = clients;
    config.on_client = on_client;
    config.trace_log.clone_from(&options.log);
    config.dump_video = std::env::var_os("AIRPLAY_DUMP_VIDEO").map(PathBuf::from);
    config.dump_audio = std::env::var_os("AIRPLAY_DUMP_AUDIO").map(PathBuf::from);
    config.audio_latency = plan.audio_latency;
    config.nohold = options.nohold;
    config.feedback_timeout = plan.feedback_timeout;
    config.ports = plan.ports;
    config.client_policy = plan.client_policy;
    config.features = plan.features;
    config.display = plan.display;
    config.on_pin = Some(Arc::new(|pin: &str| {
        console::out(&format!("*** enter PIN {pin} on the client ***"));
    }));
    config.on_artwork = outputs.artwork_callback();
    config.on_metadata = outputs.metadata_callback();
    config.on_remote_control = outputs.remote_control_callback();
    config.log_streaming_reports = options.fps_data;
    config.volume_curve = plan.volume_curve;
    config.initial_volume = plan.volume;
    config.pace_video = options.pace_video;

    let outcome = rust_airplay::run(config)
        .await
        .map_err(|error| Error::run(error.to_string()));

    // Finishing the writes that are still queued is part of stopping: a program that exits
    // with the last cover art still in the channel is a program that lost it.
    outputs.finish();
    outcome
}
