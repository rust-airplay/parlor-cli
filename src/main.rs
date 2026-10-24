// SPDX-License-Identifier: GPL-3.0-or-later
//! A command-line AirPlay receiver.
//!
//! Everything that makes a receiver a receiver is in the `rust-airplay` library: the
//! protocol, discovery, the media session and the renderer boundary. This program is the
//! policy around it — it parses arguments, decides where an identity lives, picks a
//! renderer, and prints. It contains no protocol code at all.
//!
//! See `README.md` for what the receiver does and `docs/API.md` for the library.

use std::collections::HashSet;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use rust_airplay::server::PinMode;
use rust_airplay::{AudioCodec, Renderer};

/// The receiver usage text.
const USAGE: &str = "parlor - an AirPlay receiver

Usage: parlor [options]

The names match UxPlay's command line, so a UxPlay command works here:

  -n <name>          Name clients show (default: parlor)
  -nh                Accepted; this receiver never appends a hostname
  -h265              Allow HEVC and offer 4K
  -pin [nnnn]        Ask for a PIN, optionally fixed (default: one-time PIN)
  -pw [password]     Require a SETUP password; without one, a random one is printed
  -m [mac]           Advertised hardware address (default: from the identity)
  -key [file]        Where the identity seed lives; without one nothing is stored
  -reg [file]        Where remembered clients live
  -p [n]             Upstream's fixed port layout, or a base port n
  -al <seconds>      Audio delay reported to the client (default: 0.25)
  -vol <0..1>        Volume a session starts at (default: 1.0)
  -db <low:high>     Rescale the volume slider onto this dB range
  -taper             Tapered volume profile
  -s <w>x<h>[@<r>]   Display offered to the client (default: 1920x1080@60)
  -fps <fps>         Highest frame rate asked of the client (default: 30)
  -o                 Offer an overscanned display
  -as <sink>         GStreamer audio sink
  -vs <sink>         GStreamer video sink
  -vp <parser>       GStreamer parser for the mirror stream
  -vd <decoder>      GStreamer decoder for the mirror stream
  -avdec             Force the software decoder
  -ca [file]         Report cover art, or write it to a file
  -md <file>         Write the now-playing text to a file
  -dacp [file]       Export the remote-control identifiers to a file
  -reset <seconds>   Close a session with no /feedback for n seconds (0 = never)
  -nohold            A new client takes over from the current one
  -restrict          Admit only the devices named with -allow
  -allow <id>        A device that may connect (repeatable)
  -block <id>        A device that never may (repeatable)
  -FPSdata           Log the client's streaming reports
  -q                 Only errors on the console
  -v                 Show the session's records as well
  -d                 Show everything, including protocol detail
  --log-level <l>    Off, error, warn (default), info, debug or trace
  -h, -help          Print this text

The same options spelled out, for a script that prefers words:

  --name --port --no-pin --password --device-id --seed --log --render
  --audio-sink --video-sink --video-parser --video-decoder --warm-video
  --pace-video --width --height --refresh-rate --max-fps --overscanned
  --hevc --features --restrict --allow --block --legacy-ports --timing-port
  --udp-control-port --data-port --mirror-port --nohold --reset
  --audio-latency --volume --taper --db --fps-data --cover-art --metadata
  --dacp --audio-codec --key-file --clients-file --ephemeral-seed
  --random-password --verbose --base-port

A UxPlay option this receiver cannot honour is refused with the reason, not
ignored: -hls and its tuning options (-lang, -slang, -custom, -rpi), -mp4,
-p2p, -vsync, -async, -vrtp, -artp, -v4l2, -bt709, -srgb, -fs, -f, -r, -nc,
-nofreeze and -vc.

`--render null` reads the media, decrypts and orders it, and plays nothing,
which is how a session is checked without a sound card. `--render gst` hands it
to GStreamer. A build with the `gst` feature plays through GStreamer unless told
otherwise.

Which audio coding a client sends is stated in SETUP; `--audio-codec` is only
what a session falls back to when the client never says.";

/// Everything the receiver needs before it starts.
#[derive(Debug, Clone)]
struct Options {
    name: String,
    port: u16,
    pin: PinMode,
    password: Option<String>,
    device_id: Option<String>,
    seed: Option<[u8; 32]>,
    log: Option<String>,
    render: RenderBackend,
    audio_codec: AudioCodec,
    /// Overrides for the GStreamer backend, when that is the one chosen.
    audio_sink: Option<String>,
    video_sink: Option<String>,
    video_parser: Option<String>,
    video_decoder: Option<String>,
    warm_video: bool,
    /// Whether the mirror stream is paced by its timestamps.
    pace_video: bool,
    /// Audio delay reported to the client, in seconds.
    audio_latency: f64,
    /// Whether HEVC is allowed, which is feature bit 42.
    hevc: bool,
    /// Feature bitmap to advertise, when the caller wants to set it.
    features: Option<u64>,
    /// Admit only the devices in `allow`.
    restrict: bool,
    /// Devices that may connect.
    allow: Vec<String>,
    /// Devices that never may.
    block: Vec<String>,
    /// Whether to bind upstream's fixed media port layout.
    legacy_ports: bool,
    /// Whether a new client takes over from the current one.
    nohold: bool,
    /// Seconds without feedback after which a session is closed; `0` never.
    reset_after: f64,
    /// Media ports, with `0` meaning the system picks.
    timing_port: u16,
    udp_control_port: u16,
    data_port: u16,
    mirror_port: u16,
    /// Display offered to a client in `/info`.
    width: u64,
    height: u64,
    refresh_rate: f64,
    max_fps: u64,
    overscanned: bool,
    /// Volume a session starts at, as a slider position.
    volume: f64,
    /// Whether to use the tapered volume profile.
    taper: bool,
    /// The dB range the slider is rescaled onto.
    db_low: f64,
    db_high: f64,
    /// Whether to log the client's streaming reports.
    fps_data: bool,
    /// File to write cover art to, when the user named one.
    cover_art: Option<std::path::PathBuf>,
    /// File to write now-playing metadata to.
    metadata: Option<std::path::PathBuf>,
    /// File to export the remote-control identifiers to.
    dacp: Option<std::path::PathBuf>,
    /// Whether to keep the identity in `key_file` instead of the default place.
    ephemeral_seed: bool,
    /// Where the identity seed lives, when the user named a file.
    key_file: Option<std::path::PathBuf>,
    /// Where remembered clients live, when the user named a file.
    clients_file: Option<std::path::PathBuf>,
    /// Whether to print a random password and require it, upstream's `-pw` without one.
    random_password: bool,
    /// A base port, upstream's `-p n`.
    base_port: Option<u16>,
    /// What the console shows; warnings by default.
    log_level: log::LevelFilter,
}

/// What plays the media a client sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderBackend {
    /// Read it, decrypt it, order it, and play nothing.
    Null,
    /// Hand it to GStreamer, in a build that has the backend.
    Gst,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            name: "parlor".to_owned(),
            port: 7000,
            // A current client asks for a PIN, and the PIN is printed so the user can
            // type it.
            pin: PinMode::OneTime,
            password: None,
            device_id: None,
            seed: None,
            log: None,
            render: if cfg!(feature = "gst") {
                RenderBackend::Gst
            } else {
                RenderBackend::Null
            },
            audio_sink: None,
            video_sink: None,
            video_parser: None,
            video_decoder: None,
            warm_video: false,
            pace_video: false,
            audio_latency: 0.25,
            volume: 1.0,
            taper: false,
            db_low: -30.0,
            db_high: 0.0,
            fps_data: false,
            cover_art: None,
            metadata: None,
            dacp: None,
            ephemeral_seed: false,
            key_file: None,
            clients_file: None,
            random_password: false,
            base_port: None,
            log_level: log::LevelFilter::Warn,
            hevc: false,
            features: None,
            restrict: false,
            allow: Vec::new(),
            block: Vec::new(),
            nohold: false,
            reset_after: 0.0,
            legacy_ports: false,
            timing_port: 0,
            udp_control_port: 0,
            data_port: 0,
            mirror_port: 0,
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            max_fps: 30,
            overscanned: false,

            // AirPlay 2 clients send AAC-ELD, which is what an iPhone or iPad paired with
            // this receiver sends.
            audio_codec: AudioCodec::AacEld,
        }
    }
}

/// The renderer a session plays through, chosen from the options.
///
/// `--render gst` in a build without the feature is refused rather than quietly
/// downgraded: a receiver that plays nothing while its user believes it should be playing
/// is worse than one that says which build it is.
fn renderer(backend: RenderBackend, options: &Options) -> Result<Arc<dyn Renderer>, String> {
    match backend {
        RenderBackend::Null => Ok(Arc::new(rust_airplay::NullRenderer::new())),
        RenderBackend::Gst => open_gst(options),
    }
}

/// The GStreamer backend, when this build has it.
#[cfg(feature = "gst")]
fn open_gst(options: &Options) -> Result<Arc<dyn Renderer>, String> {
    // The defaults come from the environment, and a flag overrides one of them.
    let mut backend = rust_airplay_gstream::Options::default();
    if let Some(sink) = &options.audio_sink {
        backend.audio_sink = sink.clone();
    }
    // The sink decides the decoder: replacing one without the other would ask a software
    // decoder to feed a sink that only takes hardware memory, or the reverse.
    if let Some(sink) = &options.video_sink {
        backend = backend.with_video_sink(sink);
    }
    if let Some(parser) = &options.video_parser {
        backend = backend.with_video_parser(parser);
    }
    if let Some(decoder) = &options.video_decoder {
        backend = backend.with_video_decoder(decoder);
    }
    if options.warm_video {
        backend.warm_video = true;
    }

    let program = rust_airplay_gstream::GstRenderer::find_launcher()
        .map_err(|error| format!("--render gst: {error}"))?;
    let renderer = rust_airplay_gstream::GstRenderer::with_options(&program, backend)
        .map_err(|error| format!("--render gst: {error}"))?;
    log::info!(
        "render    gst    {} -> audio {}, video {}",
        renderer.program(),
        renderer.audio_sink(),
        renderer.video_sink()
    );
    // Whether the video pipeline is already up is the difference between a picture that
    // appears at once and one that takes about a second, so it is worth saying which of
    // the two a session will get.
    log::info!(
        "render    video  {}",
        if renderer.video_is_warm() {
            "warm (started before any client, so a connection does not wait for a window)"
        } else {
            "cold (started when a client connects; AIRPLAY_GST_VIDEO_WARM=1 starts it now)"
        }
    );
    Ok(Arc::new(renderer))
}

/// The GStreamer backend, which this build does not have.
#[cfg(not(feature = "gst"))]
fn open_gst(options: &Options) -> Result<Arc<dyn Renderer>, String> {
    let _ = options;
    Err("--render gst needs a build with the `gst` feature: \
         cargo run --features gst --"
        .to_owned())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("parlor: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Starts a receiver from the command line.
#[tokio::main]
async fn run() -> Result<(), String> {
    let options = match parse_arguments(std::env::args().skip(1))? {
        Some(options) => options,
        None => {
            println!("{USAGE}");
            return Ok(());
        }
    };
    install_logger(options.log_level);

    let seed = match options.seed {
        Some(seed) => seed,
        // `-key` without a file means "do not keep an identity", which is upstream's
        // behaviour and what a test run wants: pairing works, and nothing is left behind.
        None if options.ephemeral_seed => {
            log::info!("identity  ephemeral (nothing is stored)");
            let mut seed = [0u8; 32];
            getrandom::getrandom(&mut seed).map_err(|error| format!("no random bytes: {error}"))?;
            seed
        }
        None => load_or_create_seed(options.key_file.as_deref())?,
    };
    // The registry lives here, not in the library: a receiver decides for itself how it
    // remembers clients (`docs/ARCHITECTURE.md`). The library is handed the
    // set, and a callback that writes a newly verified key.
    let clients_file = match &options.clients_file {
        Some(path) => path.clone(),
        None => clients_path()?,
    };
    let clients = Arc::new(Mutex::new(load_clients(&clients_file)));
    let on_client: rust_airplay::receiver::ClientCallback = {
        let path = clients_file.clone();
        Arc::new(move |client: &[u8; 32]| {
            if let Err(error) = record_client(&path, client) {
                eprintln!("parlor: cannot record the client: {error}");
            }
        })
    };

    // `-h265` is one bit and a bigger default display; the caller decides both.
    let hevc_bit = 1u64 << 42;
    let features = options.features.or_else(|| {
        options
            .hevc
            .then_some(rust_airplay::server::DEFAULT_FEATURES | hevc_bit)
    });
    let (width, height) = if options.hevc && options.width == 1920 && options.height == 1080 {
        (3840, 2160)
    } else {
        (options.width, options.height)
    };

    // `-pw` without a password asks for one to be drawn; upstream draws a fresh one per
    // connection, this receiver draws one for the run and prints it.
    let mut password = options.password.clone();
    if options.random_password && password.is_none() {
        let mut bytes = [0u8; 6];
        getrandom::getrandom(&mut bytes).map_err(|error| format!("no random bytes: {error}"))?;
        let drawn = hex(&bytes);
        // The drawn password is for the person at the keyboard, so it is printed whatever
        // the log level is.
        println!("Password for clients: {drawn}");
        password = Some(drawn);
    }

    let (control_port, ports) = resolve_ports(&options);
    let on_artwork = options.cover_art.as_ref().map(|path| {
        let path = path.clone();
        std::sync::Arc::new(move |mime: &str, bytes: &[u8]| {
            if let Err(error) = std::fs::write(&path, bytes) {
                eprintln!("parlor: cannot write {}: {error}", path.display());
            } else {
                eprintln!(
                    "parlor: {mime} cover art, {} bytes -> {}",
                    bytes.len(),
                    path.display()
                );
            }
        }) as rust_airplay::ArtworkCallback
    });
    let on_metadata = options.metadata.as_ref().map(|path| {
        let path = path.clone();
        std::sync::Arc::new(move |bytes: &[u8]| {
            let text = rust_airplay::media::dmap::text(bytes);
            if let Err(error) = std::fs::write(&path, &text) {
                eprintln!("parlor: cannot write {}: {error}", path.display());
            } else {
                eprintln!(
                    "parlor: metadata -> {}
{text}",
                    path.display()
                );
            }
        }) as rust_airplay::MetadataCallback
    });
    let on_remote_control = options.dacp.as_ref().map(|path| {
        let path = path.clone();
        std::sync::Arc::new(move |dacp: &str, remote: &str| {
            if let Err(error) = std::fs::write(
                &path,
                format!(
                    "{dacp}
{remote}
"
                ),
            ) {
                eprintln!("parlor: cannot write {}: {error}", path.display());
            }
        }) as rust_airplay::RemoteControlCallback
    });

    let on_pin: rust_airplay::PinCallback = Arc::new(|pin: &str| {
        println!("*** enter PIN {pin} on the client ***");
    });

    // A short banner, the way a receiver is usually introduced: what it is called, where it
    // listens, what plays, and what is already known. Nothing here needs a log level to read;
    // `-q` suppresses it because a quiet run should be quiet.
    if options.log_level >= log::LevelFilter::Warn {
        let remembered = clients.lock().map(|clients| clients.len()).unwrap_or(0);
        println!(
            "parlor {} - an AirPlay receiver
",
            env!("CARGO_PKG_VERSION")
        );
        println!("  Name      {}", options.name);
        println!(
            "  Port      {}",
            if control_port == 0 {
                "chosen by the system".to_owned()
            } else {
                control_port.to_string()
            }
        );
        println!(
            "  Playback  {}",
            match options.render {
                RenderBackend::Null => "none (media is read and played nowhere)",
                RenderBackend::Gst => "GStreamer",
            }
        );
        println!(
            "  Clients   {remembered} remembered
"
        );
        println!(
            "Waiting for a client. Press Ctrl+C to stop.
"
        );
    }

    // The renderer is chosen before anything is advertised, because a backend this build
    // or this machine cannot provide is something to say now rather than when a client is
    // already waiting for sound.
    let renderer = renderer(options.render, &options)?;
    if options.render == RenderBackend::Null {
        log::info!("render    null   media is read and played nowhere");
    }

    let config = rust_airplay::Config {
        on_pin: Some(on_pin),
        name: options.name,
        port: control_port,
        seed,
        device_id: options.device_id,
        pin: options.pin,
        password,
        audio_codec: options.audio_codec,
        renderer,
        clients,
        on_client: Some(on_client),
        trace_log: options.log.map(std::path::PathBuf::from),
        dump_video: std::env::var_os("AIRPLAY_DUMP_VIDEO").map(std::path::PathBuf::from),
        dump_audio: std::env::var_os("AIRPLAY_DUMP_AUDIO").map(std::path::PathBuf::from),
        pace_video: options.pace_video,
        audio_latency: std::time::Duration::from_secs_f64(options.audio_latency.max(0.0)),
        initial_volume: rust_airplay::Volume::new(options.volume),
        log_streaming_reports: options.fps_data,
        on_artwork,
        on_metadata,
        on_remote_control,
        volume_curve: rust_airplay::VolumeCurve {
            taper: options.taper,
            low_db: options.db_low,
            high_db: options.db_high,
        },
        features,
        nohold: options.nohold,
        feedback_timeout: (options.reset_after > 0.0)
            .then(|| std::time::Duration::from_secs_f64(options.reset_after)),
        ports,
        client_policy: rust_airplay::server::ClientPolicy {
            restrict: options.restrict,
            allow: options.allow,
            block: options.block,
        },
        display: rust_airplay::server::Display {
            width,
            height,
            refresh_rate: options.refresh_rate,
            max_fps: options.max_fps,
            overscanned: options.overscanned,
        },
    };
    rust_airplay::run(config)
        .await
        .map_err(|error| error.to_string())
}

/// Prints the library's records the way a terminal wants them.
///
/// The library emits `log` records rather than printing, because a library that writes to
/// standard output takes that decision away from the program embedding it.
fn install_logger(level: log::LevelFilter) {
    struct Terminal;

    impl log::Log for Terminal {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            if record.level() <= log::Level::Info {
                println!("{}", record.args());
            } else {
                eprintln!("{}", record.args());
            }
        }

        fn flush(&self) {}
    }

    let _ = log::set_logger(&Terminal).map(|()| log::set_max_level(level));
}

/// Reads a 32 byte seed from hex.
fn parse_seed(text: &str) -> Result<[u8; 32], String> {
    let text = text.trim();
    if text.len() != 64 {
        return Err("--seed needs 64 hex characters (32 bytes)".to_owned());
    }
    let mut seed = [0u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("--seed: {error}"))?;
    }
    Ok(seed)
}

/// Lower-case hex, for the one file this program writes.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The media ports this run binds.
///
/// `--legacy-ports` picks upstream's fixed layout; an explicit flag overrides one port of
/// it rather than replacing the layout, so `--legacy-ports --data-port 6005` is a layout
/// with one port moved.
fn media_ports(options: &Options) -> rust_airplay::media::ports::MediaPorts {
    let mut ports = if options.legacy_ports {
        rust_airplay::media::ports::MediaPorts::legacy()
    } else {
        rust_airplay::media::ports::MediaPorts::automatic()
    };
    for (value, slot) in [
        (options.timing_port, &mut ports.timing),
        (options.udp_control_port, &mut ports.control),
        (options.data_port, &mut ports.data),
        (options.mirror_port, &mut ports.mirror),
    ] {
        if value != 0 {
            *slot = value;
        }
    }
    ports
}

/// The control port and media ports this run binds.
///
/// `-p n` is upstream's layout: `tcp[0..3]` and `udp[0..3]` are `n..n+2`, and the RTSP
/// control port is `tcp[1]`, which is `n+1`. Legacy and explicit port flags keep their own
/// meaning, and a base port overrides them all because it names the whole layout.
fn resolve_ports(options: &Options) -> (u16, rust_airplay::media::ports::MediaPorts) {
    let Some(base) = options.base_port else {
        return (options.port, media_ports(options));
    };
    let ports = rust_airplay::media::ports::MediaPorts {
        timing: base,
        control: base.saturating_add(1),
        data: base.saturating_add(2),
        mirror: base,
    };
    (base.saturating_add(1), ports)
}

/// The directory this program keeps its files in.
fn data_directory() -> Result<std::path::PathBuf, String> {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "neither LOCALAPPDATA nor HOME is set".to_owned())?;
    Ok(std::path::Path::new(&base).join("parlor"))
}

/// Where `-dacp` writes when it is given no file.
///
/// Upstream writes `$HOME/.uxplay.dacp`; this program keeps its files together instead, so
/// the default is the same directory as the identity.
fn default_dacp_path() -> Result<std::path::PathBuf, String> {
    Ok(data_directory()?.join("dacp"))
}

/// Path of the identity seed inside the user's local application data.
fn seed_path() -> Result<std::path::PathBuf, String> {
    Ok(data_directory()?.join("identity.seed"))
}

/// Loads the identity seed, creating and storing one on first run.
///
/// The library never touches the filesystem. A receiver that had to be paired again after
/// every restart is what this file prevents.
fn load_or_create_seed(path: Option<&std::path::Path>) -> Result<[u8; 32], String> {
    let path = match path {
        Some(path) => path.to_path_buf(),
        None => seed_path()?,
    };
    if let Ok(text) = std::fs::read_to_string(&path) {
        return parse_seed(&text);
    }
    let mut seed = [0u8; 32];
    // `getrandom` is the same source the pairing service uses.
    getrandom::getrandom(&mut seed).map_err(|error| format!("no random bytes: {error}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(&path, hex(&seed))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    log::info!("created a new identity at {}", path.display());
    Ok(seed)
}

/// Path of the client registry.
fn clients_path() -> Result<std::path::PathBuf, String> {
    Ok(data_directory()?.join("clients"))
}

/// Reads the remembered client keys, one 64 character hex key per line.
///
/// A line that is not a key is skipped rather than failing the receiver: a file somebody
/// edited by hand must not stop a receiver from starting.
fn load_clients(path: &std::path::Path) -> HashSet<[u8; 32]> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    text.lines()
        .filter_map(|line| parse_seed(line).ok())
        .collect()
}

/// Appends a client key, creating the file if it is the first one.
fn record_client(path: &std::path::Path, client: &[u8; 32]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    use std::io::Write;
    writeln!(file, "{}", hex(client))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Parses the command line, or returns `None` when the user asked for help.
fn parse_arguments(arguments: impl Iterator<Item = String>) -> Result<Option<Options>, String> {
    let mut options = Options::default();
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" | "-help" => return Ok(None),

            // UxPlay's names. A command written for it runs here unchanged for everything
            // this receiver can honour.
            "-n" => options.name = next_value(&mut arguments, "-n")?,
            // Accepted, and there is nothing to do: a name is the name, and this receiver
            // never appends a hostname to it.
            "-nh" => {}
            "-h265" => options.hevc = true,
            "-pin" => {
                options.pin = match optional_value(&mut arguments) {
                    Some(digits) => {
                        let digits = digits
                            .parse()
                            .map_err(|_| format!("-pin: {digits} is not a PIN"))?;
                        PinMode::Fixed(pin_option("-pin", digits)?)
                    }
                    None => PinMode::OneTime,
                };
            }
            "-pw" => match optional_value(&mut arguments) {
                Some(password) => {
                    options.password = Some(password_option("-pw", password)?);
                }
                // Upstream draws a password at each connection; this receiver prints one
                // and keeps it for the run, which is the same promise to the user.
                None => options.random_password = true,
            },
            "-m" => {
                if let Some(value) = optional_value(&mut arguments) {
                    options.device_id = Some(value);
                }
            }
            "-key" => match optional_value(&mut arguments) {
                Some(path) => options.key_file = Some(std::path::PathBuf::from(path)),
                None => options.ephemeral_seed = true,
            },
            "-reg" => {
                if let Some(path) = optional_value(&mut arguments) {
                    options.clients_file = Some(std::path::PathBuf::from(path));
                }
            }
            "-p" => match optional_value(&mut arguments) {
                // Upstream also accepts `-p tcp <list>` and `-p udp <list>`; a list is
                // refused rather than read as a port, so the message says what happened.
                Some(kind) if kind == "tcp" || kind == "udp" => {
                    return Err(format!(
                        "-p {kind} <list> is not supported; this receiver takes -p or -p <base>

{USAGE}"
                    ));
                }
                Some(base) => {
                    let base: u16 = base
                        .parse()
                        .map_err(|_| format!("-p: {base} is not a port"))?;
                    // The layout is `n..n+2`, so the last port has to stay a port.
                    options.base_port = Some(within("-p", base, 1, 65_533)?);
                }
                None => options.legacy_ports = true,
            },
            "-al" => {
                let value = next_value(&mut arguments, "-al")?
                    .parse()
                    .map_err(|_| "-al: not a number of seconds".to_owned())?;
                options.audio_latency = within("-al", value, 0.0, 10.0)?;
            }
            "-vol" => {
                let value = next_value(&mut arguments, "-vol")?
                    .parse()
                    .map_err(|_| "-vol: not a number".to_owned())?;
                options.volume = within("-vol", value, 0.0, 1.0)?;
            }
            "-db" => {
                let value = next_value(&mut arguments, "-db")?;
                let (low, high) = value
                    .split_once(':')
                    .ok_or_else(|| format!("-db: {value} is not low:high"))?;
                options.db_low = low
                    .parse()
                    .map_err(|_| format!("-db: {low} is not a number"))?;
                options.db_high = high
                    .parse()
                    .map_err(|_| format!("-db: {high} is not a number"))?;
                if options.db_low >= options.db_high
                    || !options.db_low.is_finite()
                    || !options.db_high.is_finite()
                {
                    return Err(format!(
                        "-db: the range must be low:high with low below high, got {value}"
                    ));
                }
            }
            "-taper" => options.taper = true,
            "-s" => {
                let (width, height, rate) = parse_display(&next_value(&mut arguments, "-s")?)?;
                options.width = width;
                options.height = height;
                if let Some(rate) = rate {
                    options.refresh_rate = rate;
                }
            }
            "-fps" => {
                let value = next_value(&mut arguments, "-fps")?
                    .parse()
                    .map_err(|_| "-fps: not a number".to_owned())?;
                options.max_fps = within("-fps", value, 1, 240)?;
            }
            "-o" => options.overscanned = true,
            "-as" => options.audio_sink = Some(next_value(&mut arguments, "-as")?),
            "-vs" => options.video_sink = Some(next_value(&mut arguments, "-vs")?),
            "-vp" => options.video_parser = Some(next_value(&mut arguments, "-vp")?),
            "-vd" => options.video_decoder = Some(next_value(&mut arguments, "-vd")?),
            "-avdec" => {
                options.video_decoder =
                    Some("avdec_h264 ! videoconvert ! video/x-raw,format=I420".to_owned());
            }
            "-ca" => {
                options.cover_art = optional_value(&mut arguments).map(std::path::PathBuf::from);
            }
            "-md" => {
                options.metadata =
                    Some(std::path::PathBuf::from(next_value(&mut arguments, "-md")?));
            }
            "-dacp" => {
                options.dacp = Some(match optional_value(&mut arguments) {
                    Some(path) => std::path::PathBuf::from(path),
                    None => default_dacp_path()?,
                });
            }
            "-reset" => {
                let value: f64 = next_value(&mut arguments, "-reset")?
                    .parse()
                    .map_err(|_| "-reset: not a number of seconds".to_owned())?;
                if !value.is_finite() || value < 0.0 {
                    return Err(format!("-reset: {value} is not a number of seconds"));
                }
                options.reset_after = value;
            }
            "-nohold" => options.nohold = true,
            "-restrict" => options.restrict = true,
            "-allow" => options.allow.push(next_value(&mut arguments, "-allow")?),
            "-block" => options.block.push(next_value(&mut arguments, "-block")?),
            "-FPSdata" => options.fps_data = true,
            "-d" => options.log_level = log::LevelFilter::Debug,
            "-v" => options.log_level = log::LevelFilter::Info,
            "-q" => options.log_level = log::LevelFilter::Error,

            // Options this receiver cannot honour are refused with the reason. A UxPlay
            // command that asks for HLS or for recording has to fail where the user can see
            // it, not play a session that quietly is not what was asked for.
            "-hls" | "-vsync" | "-async" | "-bt709" | "-srgb" | "-fs" | "-nc" | "-nofreeze"
            | "-scrsv" | "-p2p" => {
                return Err(format!(
                    "{} is not supported by this receiver

{USAGE}",
                    argument
                ));
            }
            "-lang" | "-slang" | "-custom" | "-rpi" | "-mp4" | "-vrtp" | "-artp" | "-vc"
            | "-v4l2" | "-f" | "-r" => {
                return Err(format!(
                    "{} is not supported by this receiver (its feature is not implemented)

{USAGE}",
                    argument
                ));
            }

            "--key-file" => {
                options.key_file = Some(std::path::PathBuf::from(next_value(
                    &mut arguments,
                    "--key-file",
                )?));
            }
            "--clients-file" => {
                options.clients_file = Some(std::path::PathBuf::from(next_value(
                    &mut arguments,
                    "--clients-file",
                )?));
            }
            "--ephemeral-seed" => options.ephemeral_seed = true,
            "--random-password" => options.random_password = true,
            "--verbose" => options.log_level = log::LevelFilter::Info,
            "--quiet" => options.log_level = log::LevelFilter::Error,
            "--log-level" => {
                let level = next_value(&mut arguments, "--log-level")?;
                options.log_level = parse_level(&level)?;
            }
            "--base-port" => {
                let value = next_value(&mut arguments, "--base-port")?
                    .parse()
                    .map_err(|_| "--base-port: not a port".to_owned())?;
                options.base_port = Some(within("--base-port", value, 1, 65_533)?);
            }

            "--name" => options.name = next_value(&mut arguments, "--name")?,
            "--port" => {
                options.port = next_value(&mut arguments, "--port")?
                    .parse()
                    .map_err(|error| format!("--port: {error}"))?;
            }
            "--no-pin" => options.pin = PinMode::Disabled,
            "--pin" => {
                // The digits are optional: without them the receiver draws a one-time PIN
                // and prints it when a client asks for one.
                options.pin = match arguments.peek() {
                    Some(value) if !value.starts_with('-') => {
                        let digits: u16 = next_value(&mut arguments, "--pin")?
                            .parse()
                            .map_err(|error| format!("--pin: {error}"))?;
                        PinMode::Fixed(pin_option("--pin", digits)?)
                    }
                    _ => PinMode::OneTime,
                };
            }
            "--password" => {
                let value = next_value(&mut arguments, "--password")?;
                options.password = Some(password_option("--password", value)?);
            }
            "--device-id" => options.device_id = Some(next_value(&mut arguments, "--device-id")?),
            "--seed" => options.seed = Some(parse_seed(&next_value(&mut arguments, "--seed")?)?),
            "--log" => options.log = Some(next_value(&mut arguments, "--log")?),
            "--render" => {
                let name = next_value(&mut arguments, "--render")?;
                options.render = match name.as_str() {
                    "null" => RenderBackend::Null,
                    "gst" => RenderBackend::Gst,
                    other => {
                        return Err(format!("--render: {other} is not a backend (null or gst)"));
                    }
                };
            }
            "--audio-sink" => {
                options.audio_sink = Some(next_value(&mut arguments, "--audio-sink")?)
            }
            "--video-sink" => {
                options.video_sink = Some(next_value(&mut arguments, "--video-sink")?)
            }
            "--video-parser" => {
                options.video_parser = Some(next_value(&mut arguments, "--video-parser")?);
            }
            "--video-decoder" => {
                options.video_decoder = Some(next_value(&mut arguments, "--video-decoder")?);
            }
            "--warm-video" => options.warm_video = true,
            "--pace-video" => options.pace_video = true,
            "--width" => {
                let value = next_value(&mut arguments, "--width")?
                    .parse()
                    .map_err(|_| "--width: not a number".to_owned())?;
                options.width = within("--width", value, 1, 16_384)?;
            }
            "--height" => {
                let value = next_value(&mut arguments, "--height")?
                    .parse()
                    .map_err(|_| "--height: not a number".to_owned())?;
                options.height = within("--height", value, 1, 16_384)?;
            }
            "--refresh-rate" => {
                let value: f64 = next_value(&mut arguments, "--refresh-rate")?
                    .parse()
                    .map_err(|_| "--refresh-rate: not a number".to_owned())?;
                if !value.is_finite() || value <= 0.0 {
                    return Err(format!("--refresh-rate: {value} is not a refresh rate"));
                }
                options.refresh_rate = value;
            }
            "--max-fps" => {
                let value = next_value(&mut arguments, "--max-fps")?
                    .parse()
                    .map_err(|_| "--max-fps: not a number".to_owned())?;
                options.max_fps = within("--max-fps", value, 1, 240)?;
            }
            "--overscanned" => options.overscanned = true,
            "--nohold" => options.nohold = true,
            "--reset" => {
                let value = next_value(&mut arguments, "--reset")?;
                let seconds: f64 = value
                    .parse()
                    .map_err(|_| format!("--reset: {value} is not a number of seconds"))?;
                if !seconds.is_finite() || seconds < 0.0 {
                    return Err(format!("--reset: {value} is not a number of seconds"));
                }
                options.reset_after = seconds;
            }
            "--legacy-ports" => options.legacy_ports = true,
            "--timing-port" => {
                options.timing_port = next_value(&mut arguments, "--timing-port")?
                    .parse()
                    .map_err(|_| "--timing-port: not a port".to_owned())?;
            }
            "--udp-control-port" => {
                options.udp_control_port = next_value(&mut arguments, "--udp-control-port")?
                    .parse()
                    .map_err(|_| "--udp-control-port: not a port".to_owned())?;
            }
            "--data-port" => {
                options.data_port = next_value(&mut arguments, "--data-port")?
                    .parse()
                    .map_err(|_| "--data-port: not a port".to_owned())?;
            }
            "--mirror-port" => {
                options.mirror_port = next_value(&mut arguments, "--mirror-port")?
                    .parse()
                    .map_err(|_| "--mirror-port: not a port".to_owned())?;
            }
            "--restrict" => options.restrict = true,
            "--allow" => options.allow.push(next_value(&mut arguments, "--allow")?),
            "--block" => options.block.push(next_value(&mut arguments, "--block")?),
            "--hevc" => options.hevc = true,
            "--features" => {
                let value = next_value(&mut arguments, "--features")?;
                let digits = value.trim_start_matches("0x");
                options.features = Some(
                    u64::from_str_radix(digits, 16)
                        .map_err(|_| format!("--features: {value} is not hex"))?,
                );
            }
            "--audio-latency" => {
                let value = next_value(&mut arguments, "--audio-latency")?;
                let seconds: f64 = value
                    .parse()
                    .map_err(|_| format!("--audio-latency: {value} is not a number"))?;
                options.audio_latency = within("--audio-latency", seconds, 0.0, 10.0)?;
            }
            "--taper" => options.taper = true,
            "--fps-data" => options.fps_data = true,
            "--cover-art" => {
                // The path is optional: upstream's `-ca` without one shows the art, which a
                // command-line receiver can only report.
                options.cover_art = arguments.peek().and_then(|next| {
                    (!next.starts_with('-')).then(|| std::path::PathBuf::from(next.clone()))
                });
                if options.cover_art.is_some() {
                    let _ = arguments.next();
                }
            }
            "--metadata" => {
                options.metadata = Some(std::path::PathBuf::from(next_value(
                    &mut arguments,
                    "--metadata",
                )?));
            }
            "--dacp" => {
                options.dacp = Some(std::path::PathBuf::from(next_value(
                    &mut arguments,
                    "--dacp",
                )?));
            }
            "--db" => {
                let value = next_value(&mut arguments, "--db")?;
                let (low, high) = value
                    .split_once(':')
                    .ok_or_else(|| format!("--db: {value} is not low:high"))?;
                options.db_low = low
                    .parse()
                    .map_err(|_| format!("--db: {low} is not a number"))?;
                options.db_high = high
                    .parse()
                    .map_err(|_| format!("--db: {high} is not a number"))?;
                if options.db_low >= options.db_high
                    || !options.db_low.is_finite()
                    || !options.db_high.is_finite()
                {
                    return Err(format!(
                        "--db: the range must be low:high with low below high, got {value}"
                    ));
                }
            }
            "--volume" => {
                let value = next_value(&mut arguments, "--volume")?;
                let level: f64 = value
                    .parse()
                    .map_err(|_| format!("--volume: {value} is not a number"))?;
                options.volume = within("--volume", level, 0.0, 1.0)?;
            }

            "--audio-codec" => {
                let name = next_value(&mut arguments, "--audio-codec")?;
                options.audio_codec = match name.as_str() {
                    "alac" => AudioCodec::Alac,
                    "aac-eld" => AudioCodec::AacEld,
                    other => {
                        return Err(format!(
                            "--audio-codec: {other} is not a coding (alac or aac-eld)"
                        ));
                    }
                };
            }
            other => return Err(format!("unknown option {other}\n\n{USAGE}")),
        }
    }
    Ok(Some(options))
}

fn next_value(
    arguments: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    option: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{option} needs a value"))
}

/// Takes the next argument when it is a value rather than another option.
///
/// Upstream's optional-value options (`-pin`, `-pw`, `-key`, `-ca`, `-dacp`) are written
/// `-pin 1234` or `-pin`, and the difference is whether the next word starts with a dash.
fn optional_value(
    arguments: &mut std::iter::Peekable<impl Iterator<Item = String>>,
) -> Option<String> {
    match arguments.peek() {
        Some(next) if !next.starts_with('-') => arguments.next(),
        _ => None,
    }
}

/// Checks a numeric option is inside the range the protocol or the backend allows.
fn within<T: PartialOrd + Copy + std::fmt::Display>(
    option: &str,
    value: T,
    low: T,
    high: T,
) -> Result<T, String> {
    if value < low || value > high {
        return Err(format!("{option}: {value} is outside {low}..={high}"));
    }
    Ok(value)
}

/// Checks a PIN is one a person can type: four digits, 0001 to 9999.
///
/// Upstream refuses anything outside that range when it parses the option, and a fixed PIN
/// outside it is a receiver no client can pair with.
fn pin_option(option: &str, digits: u16) -> Result<u16, String> {
    if !(1..=9999).contains(&digits) {
        return Err(format!("{option}: {digits} is not a PIN (0001 to 9999)"));
    }
    Ok(digits)
}

/// Checks a password against the minimum length upstream enforces.
fn password_option(option: &str, value: String) -> Result<String, String> {
    if value.chars().count() < 4 {
        return Err(format!(
            "{option}: a password needs at least four characters"
        ));
    }
    Ok(value)
}

/// Reads a log level by name.
fn parse_level(name: &str) -> Result<log::LevelFilter, String> {
    Ok(match name {
        "off" => log::LevelFilter::Off,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        other => {
            return Err(format!(
                "--log-level: {other} is not a level (off, error, warn, info, debug, trace)"
            ));
        }
    })
}

/// Reads a display the way upstream writes one: `1920x1080` or `1920x1080@60`.
fn parse_display(text: &str) -> Result<(u64, u64, Option<f64>), String> {
    let (size, rate) = match text.split_once('@') {
        Some((size, rate)) => {
            let rate: f64 = rate
                .parse()
                .map_err(|_| format!("-s: {rate} is not a refresh rate"))?;
            (size, Some(rate))
        }
        None => (text, None),
    };
    let (width, height) = size
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("-s: {size} is not <width>x<height>"))?;
    let width: u64 = width
        .parse()
        .map_err(|_| format!("-s: {width} is not a width"))?;
    let height: u64 = height
        .parse()
        .map_err(|_| format!("-s: {height} is not a height"))?;
    if width == 0 || height == 0 || rate.is_some_and(|rate| !rate.is_finite() || rate <= 0.0) {
        return Err(format!("-s: {text} is not a usable display"));
    }
    Ok((width, height, rate))
}

#[cfg(test)]
mod tests {
    use super::{Options, PinMode, RenderBackend, parse_arguments, resolve_ports};

    fn parse(arguments: &[&str]) -> Result<Options, String> {
        parse_arguments(arguments.iter().map(|argument| (*argument).to_owned()))
            .map(|options| options.expect("not a help request"))
    }

    #[test]
    fn a_uxplay_command_line_reaches_the_same_options() {
        // The point of the compatibility names: a command written for UxPlay runs here.
        let options = parse(&[
            "-n",
            "Parlor",
            "-pin",
            "1234",
            "-s",
            "1280x720@30",
            "-fps",
            "24",
            "-o",
            "-al",
            "0.5",
            "-vol",
            "0.8",
            "-taper",
            "-db",
            "-20:0",
            "-nohold",
            "-restrict",
            "-allow",
            "AABBCCDDEEFF",
            "-FPSdata",
        ])
        .expect("parsed");
        assert_eq!(options.name, "Parlor");
        assert_eq!(options.pin, PinMode::Fixed(1234));
        assert_eq!((options.width, options.height), (1280, 720));
        assert_eq!(options.refresh_rate, 30.0);
        assert_eq!(options.max_fps, 24);
        assert!(options.overscanned);
        assert_eq!(options.audio_latency, 0.5);
        assert_eq!(options.volume, 0.8);
        assert!(options.taper);
        assert_eq!((options.db_low, options.db_high), (-20.0, 0.0));
        assert!(options.nohold);
        assert!(options.restrict);
        assert_eq!(options.allow, vec!["AABBCCDDEEFF".to_owned()]);
        assert!(options.fps_data);
    }

    #[test]
    fn the_port_option_keeps_upstreams_layout() {
        // `-p` alone is the fixed layout; `-p n` puts tcp and udp at n..n+2 and the control
        // port at n+1 (`uxplay.cpp`, `-p` handling).
        let fixed = parse(&["-p"]).expect("parsed");
        assert!(fixed.legacy_ports);
        assert!(fixed.base_port.is_none());

        let based = parse(&["-p", "7000"]).expect("parsed");
        assert_eq!(based.base_port, Some(7000));
        let (control, ports) = resolve_ports(&based);
        assert_eq!(control, 7001);
        assert_eq!(
            (ports.mirror, ports.timing, ports.control, ports.data),
            (7000, 7000, 7001, 7002)
        );
    }

    #[test]
    fn key_and_password_take_an_optional_value() {
        // `-key` with no file means the identity is not stored; `-pw` with no password means
        // one is drawn and printed.
        let ephemeral = parse(&["-key"]).expect("parsed");
        assert!(ephemeral.ephemeral_seed);
        assert!(ephemeral.key_file.is_none());

        let stored = parse(&["-key", "seed.txt"]).expect("parsed");
        assert!(!stored.ephemeral_seed);
        assert_eq!(stored.key_file, Some(std::path::PathBuf::from("seed.txt")));

        let drawn = parse(&["-pw"]).expect("parsed");
        assert!(drawn.random_password);
        assert!(drawn.password.is_none());

        let given = parse(&["-pw", "hunter2"]).expect("parsed");
        assert_eq!(given.password.as_deref(), Some("hunter2"));
    }

    #[test]
    fn an_option_this_receiver_cannot_honour_is_refused_with_a_reason() {
        let error = parse(&["-hls"]).expect_err("refused");
        assert!(error.contains("not supported"), "{error}");
        let error = parse(&["-mp4", "out.mp4"]).expect_err("refused");
        assert!(error.contains("not supported"), "{error}");
    }

    #[test]
    fn options_outside_their_range_are_refused() {
        // Each of these was accepted before the audit and either clamped silently or
        // produced a value the protocol cannot use: a PIN of zero, a password shorter than
        // upstream's minimum, a volume above unity, a negative timeout, a display with no
        // pixels, a port layout that runs past 65535.
        for arguments in [
            &["-pin", "0"][..],
            &["-pin", "10000"],
            &["-pw", "abc"],
            &["-vol", "1.5"],
            &["-al", "11"],
            &["-db", "0:-30"],
            &["-s", "0x1080"],
            &["-s", "1920x0"],
            &["-fps", "0"],
            &["-reset", "-1"],
            &["-p", "70000"],
            &["--volume", "-0.1"],
            &["--audio-latency", "10.5"],
            &["--db", "-30:-40"],
            &["--base-port", "65535"],
            &["--password", "abc"],
            &["--pin", "0"],
        ] {
            assert!(parse(arguments).is_err(), "{arguments:?} was accepted");
        }
    }

    #[test]
    fn values_inside_their_range_still_parse() {
        // The boundaries are where a valid value stops being valid, not one short of it.
        let options = parse(&[
            "-pin",
            "9999",
            "-pw",
            "abcd",
            "-vol",
            "1",
            "-al",
            "10",
            "-db",
            "-30:0",
            "-s",
            "1920x1080@60",
            "-fps",
            "240",
            "-reset",
            "0",
            "-p",
            "65533",
        ])
        .expect("parsed");
        assert_eq!(options.pin, PinMode::Fixed(9999));
        assert_eq!(options.volume, 1.0);
        assert_eq!(options.audio_latency, 10.0);
        assert_eq!(options.base_port, Some(65_533));
    }

    #[test]
    fn the_log_level_is_a_setting_and_warnings_are_the_default() {
        // The device run showed that a receiver printing every media record is unreadable:
        // the session's own lines are opt-in.
        assert_eq!(Options::default().log_level, log::LevelFilter::Warn);
        assert_eq!(
            parse(&["-v"]).expect("parsed").log_level,
            log::LevelFilter::Info
        );
        assert_eq!(
            parse(&["-d"]).expect("parsed").log_level,
            log::LevelFilter::Debug
        );
        assert_eq!(
            parse(&["-q"]).expect("parsed").log_level,
            log::LevelFilter::Error
        );
        assert_eq!(
            parse(&["--log-level", "trace"]).expect("parsed").log_level,
            log::LevelFilter::Trace
        );
        assert!(parse(&["--log-level", "loud"]).is_err());
    }

    #[test]
    fn a_build_with_the_backend_plays_by_default() {
        assert_eq!(
            Options::default().render,
            if cfg!(feature = "gst") {
                RenderBackend::Gst
            } else {
                RenderBackend::Null
            }
        );
    }
}
