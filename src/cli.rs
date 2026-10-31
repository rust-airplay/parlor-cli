// SPDX-License-Identifier: GPL-3.0-or-later
//! The command line: what this program accepts, and what it refuses.
//!
//! Parsing is the one place a wrong default becomes a wrong receiver, so every value is
//! checked where it is read and the error names the option that was wrong. The parser
//! touches no filesystem, no network and no environment: it turns words into [Options]
//! and nothing else, which is what makes it testable.

use std::net::IpAddr;
use std::path::PathBuf;

use rust_airplay::AudioCodec;
use rust_airplay::server::PinMode;

use crate::Error;
use crate::files::parse_hex32;

/// The receiver usage text.
pub const USAGE: &str = r#"parlor - an AirPlay receiver

Usage: parlor [options]

The short names are the familiar ones for an AirPlay receiver:

  -n <name>          Name clients show (default: parlor)
  -nh                Accepted; this receiver never appends a hostname
  -h265              Allow HEVC and offer 4K
  -pin [nnnn]        Ask for a PIN, optionally fixed (default: one-time PIN)
  -pw [password]     Require a SETUP password; without one, a random one is printed
  -m [mac]           Advertised hardware address (default: from the identity)
  -key [file]        Where the identity seed lives; without a file nothing is stored
  -reg [file]        Where remembered clients live
  -p [n]             Upstream fixed port layout, or a base port n
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
  -FPSdata           Log the client streaming reports
  -q                 Only errors on the console
  -v                 Show the session records as well
  -d                 Show everything, including protocol detail
  --log-level <l>    Off, error, warn (default), info, debug or trace
  -V, --version      Print the version
  -h, -help          Print this text

The same options spelled out, for a script that prefers words:

  --name --port --bind --no-advertise --no-pin --pin --password --password-file
  --device-id --seed --seed-file --key-file --ephemeral-seed --clients-file
  --data-dir --log --render --audio-sink --video-sink --video-parser
  --video-decoder --video-decoder-hevc --warm-video --pace-video --no-pace-video
  --width --height --refresh-rate
  --max-fps --overscanned --hevc --features --restrict --allow --block
  --legacy-ports --base-port --timing-port --udp-control-port --data-port
  --mirror-port --nohold --reset --audio-latency --volume --taper --db
  --fps-data --cover-art --metadata --dacp --audio-codec --random-password
  --verbose --quiet --log-level --help --version

"--render null" reads the media, decrypts and orders it, and plays nothing,
which is how a session is checked without a sound card. "--render gst" hands it
to GStreamer. A build with the "gst" feature plays through GStreamer unless told
otherwise.

Which audio coding a client sends is stated in SETUP; --audio-codec is only
what a session falls back to when the client never says.

Exit codes: 0 success, 1 a run that could not start or that failed, 2 a command
line this program cannot accept."#;

/// What a session plays through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    /// Read the media, order it, and play nothing.
    Null,
    /// Hand it to GStreamer, in a build that has the backend.
    Gst,
}

/// Where the identity seed comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// The file this program keeps in its data directory.
    Default,
    /// A file the caller named.
    File(PathBuf),
    /// A seed fixed on the command line.
    Given([u8; 32]),
    /// A random seed, kept only for this run.
    Ephemeral,
}

/// How a client authenticates with a password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Password {
    /// No password; a PIN is the only gate.
    None,
    /// A password the caller supplied.
    Fixed(String),
    /// A password drawn once, printed, and kept for the run.
    Random,
    /// A password read from a file, or from standard input for a dash.
    FromFile(PathBuf),
}

/// Where the remembered-client registry lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientsSource {
    /// The file this program keeps in its data directory.
    Default,
    /// A file the caller named.
    File(PathBuf),
    /// Kept in memory only: pairing works, and nothing is written.
    Memory,
}

/// What to do with the cover art a client sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverArt {
    /// Report each image's content type and size on the console.
    Report,
    /// Write each image to this path.
    File(PathBuf),
}

/// Individual ports moved off the layout they would otherwise take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PortOverrides {
    /// The timing port, when it was moved.
    pub timing: Option<u16>,
    /// The UDP control port, when it was moved.
    pub control: Option<u16>,
    /// The data port, when it was moved.
    pub data: Option<u16>,
    /// The mirror port, when it was moved.
    pub mirror: Option<u16>,
}

impl PortOverrides {
    /// Whether any port was moved.
    #[must_use]
    pub fn any(&self) -> bool {
        self.timing.is_some()
            || self.control.is_some()
            || self.data.is_some()
            || self.mirror.is_some()
    }
}

/// The ports the media session binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ports {
    /// Let the system choose.
    Default,
    /// The protocol's fixed layout, with optional single-port overrides.
    Legacy(PortOverrides),
    /// The protocol's layout starting at this base port.
    Base(u16),
}

/// The GStreamer backend's own settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GstOptions {
    /// The audio sink.
    pub audio_sink: Option<String>,
    /// The video sink.
    pub video_sink: Option<String>,
    /// The parser for the mirror stream.
    pub video_parser: Option<String>,
    /// The decoder for an H.264 mirror stream.
    pub video_decoder: Option<String>,
    /// The decoder for an H.265 mirror stream.
    ///
    /// The two codings need two elements: a hardware decoder decodes one of them, so a
    /// receiver that offers HEVC and names only -vd still decodes HEVC in software.
    pub video_decoder_hevc: Option<String>,
    /// Use the software decoder for every coding the backend can decode.
    pub software_decoder: bool,
    /// Start the video pipeline before any client connects.
    pub warm_video: bool,
}

impl GstOptions {
    /// Whether any setting here only means something to the GStreamer backend.
    #[must_use]
    pub fn any(&self) -> bool {
        self.audio_sink.is_some()
            || self.video_sink.is_some()
            || self.video_parser.is_some()
            || self.video_decoder.is_some()
            || self.video_decoder_hevc.is_some()
            || self.software_decoder
            || self.warm_video
    }
}

/// The display offered to a client.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayOptions {
    /// Width in pixels.
    pub width: u64,
    /// Height in pixels.
    pub height: u64,
    /// Refresh rate in hertz.
    pub refresh_rate: f64,
    /// Highest frame rate asked of the client.
    pub max_fps: u16,
    /// Offer an overscanned display.
    pub overscanned: bool,
    /// Whether the size was stated rather than defaulted, so -h265 leaves it alone.
    pub size_set: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            max_fps: 30,
            overscanned: false,
            size_set: false,
        }
    }
}

/// Everything the receiver needs before it starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// The name clients see.
    pub name: String,
    /// The control port; zero lets the system choose one.
    pub port: u16,
    /// The address to bind, when the caller named one.
    pub bind: Option<IpAddr>,
    /// Whether to announce over mDNS.
    pub advertise: bool,
    /// How a client authenticates.
    pub pin: PinMode,
    /// Where the identity seed comes from.
    pub identity: Identity,
    /// Where remembered clients live.
    pub clients: ClientsSource,
    /// How a password is obtained.
    pub password: Password,
    /// The device identifier to announce.
    pub device_id: Option<String>,
    /// Where to append the request trace.
    pub log: Option<PathBuf>,
    /// The console level.
    pub log_level: log::LevelFilter,
    /// What plays the media.
    pub render: RenderBackend,
    /// The coding a session falls back to.
    pub audio_codec: AudioCodec,
    /// The GStreamer backend's settings.
    pub gst: GstOptions,
    /// The audio delay reported to the client, in seconds.
    pub audio_latency: f64,
    /// Whether HEVC is allowed, which is feature bit 42.
    pub hevc: bool,
    /// A feature bitmap the caller set itself.
    pub features: Option<u64>,
    /// Whether only the devices in the allow list may connect.
    pub restrict: bool,
    /// Devices that may connect.
    pub allow: Vec<String>,
    /// Devices that never may.
    pub block: Vec<String>,
    /// The media ports.
    pub ports: Ports,
    /// Whether a new client takes over from the current one.
    pub nohold: bool,
    /// Seconds without feedback after which a session is closed; zero never.
    pub reset_after: f64,
    /// What to do with cover art.
    pub cover_art: Option<CoverArt>,
    /// Where to write the now-playing text.
    pub metadata: Option<PathBuf>,
    /// Where to write the remote-control identifiers; the inner value is the default path.
    pub dacp: Option<Option<PathBuf>>,
    /// Whether the volume profile is tapered.
    pub taper: bool,
    /// The decibel range the slider is rescaled onto.
    pub db: (f64, f64),
    /// The volume a session starts at.
    pub volume: f64,
    /// Whether the client streaming reports are logged.
    pub fps_data: bool,
    /// The display offered to a client.
    pub display: DisplayOptions,
    /// Whether this program paces the mirror stream itself.
    ///
    /// Off by default: frames then go to the renderer as fast as it takes them, which is
    /// the shortest path from the device to the screen. On, each frame is held back to the
    /// spacing it was captured at, which spends a little latency to smooth a stream that
    /// arrives in bursts. 0.3.0's receiver did the pacing in the backend and drifted behind
    /// the stream forever; 0.3.1 leaves the choice here, where it belongs.
    pub pace_video: bool,
    /// The directory this program keeps its files in, when the caller named one.
    pub data_dir: Option<PathBuf>,
    /// Things worth telling the user about the command line they gave.
    pub warnings: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            name: "parlor".to_owned(),
            port: 7000,
            bind: None,
            advertise: true,
            // A current client asks for a PIN, and the PIN is printed so the user can
            // type it.
            pin: PinMode::OneTime,
            identity: Identity::Default,
            clients: ClientsSource::Default,
            password: Password::None,
            device_id: None,
            log: None,
            log_level: log::LevelFilter::Warn,
            render: if cfg!(feature = "gst") {
                RenderBackend::Gst
            } else {
                RenderBackend::Null
            },
            audio_codec: AudioCodec::AacEld,
            gst: GstOptions::default(),
            audio_latency: 0.25,
            hevc: false,
            features: None,
            restrict: false,
            allow: Vec::new(),
            block: Vec::new(),
            ports: Ports::Default,
            nohold: false,
            reset_after: 0.0,
            cover_art: None,
            metadata: None,
            dacp: None,
            taper: false,
            db: (-30.0, 0.0),
            volume: 1.0,
            fps_data: false,
            display: DisplayOptions::default(),
            // Frames go out as fast as the renderer takes them; -pace-video trades a
            // little latency for smoothness on a stream that arrives in bursts.
            pace_video: false,
            data_dir: None,
            warnings: Vec::new(),
        }
    }
}

/// What a command line asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    /// Run a receiver with these options.
    Run(Box<Options>),
    /// Print the usage text and exit successfully.
    Help,
    /// Print the version and exit successfully.
    Version,
}

/// The highest resolution this receiver will offer, in either direction.
pub const MAX_DISPLAY: u64 = 16_384;

/// The highest refresh rate, in hertz, this receiver will offer.
pub const MAX_REFRESH: f64 = 240.0;

/// The longest feedback timeout, in seconds, this receiver will accept.
///
/// A day is long past "never reconnect" and short of the point where the duration cannot
/// be represented; the protocol's own default is fifteen seconds.
pub const MAX_RESET: f64 = 86_400.0;

/// Parses a command line.
///
/// # Errors
///
/// Returns [Error::Usage] when a word is not an option, an option has no value, or a value
/// is outside what the receiver can honour.
pub fn parse_arguments(arguments: impl Iterator<Item = String>) -> Result<Parsed, Error> {
    let mut options = Options::default();
    let mut arguments = arguments.peekable();
    let mut base_port: Option<u16> = None;
    let mut overrides = PortOverrides::default();
    let mut legacy_ports = false;
    let mut port_named = false;
    let mut password_file: Option<PathBuf> = None;
    let mut seed_file: Option<PathBuf> = None;
    let mut seed_given: Option<[u8; 32]> = None;
    let mut ephemeral_seed = false;
    let mut key_file: Option<PathBuf> = None;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" | "-help" => return Ok(Parsed::Help),
            "--version" | "-V" => return Ok(Parsed::Version),

            // The reference's names. A command written for it runs here unchanged for
            // everything this receiver can honour.
            "-n" => options.name = next_word(&mut arguments, "-n")?,
            // Accepted, and there is nothing to do: a name is the name, and this receiver
            // never appends a hostname to it.
            "-nh" => {}
            "-h265" => options.hevc = true,
            "-pin" => {
                options.pin = match optional_value(&mut arguments) {
                    Some(digits) => {
                        let digits: u16 = digits
                            .parse()
                            .map_err(|_| Error::usage(format!("-pin: {digits} is not a PIN")))?;
                        PinMode::Fixed(pin_option("-pin", digits)?)
                    }
                    None => PinMode::OneTime,
                };
            }
            "-pw" => match optional_value(&mut arguments) {
                Some(password) => {
                    options.password = Password::Fixed(password_option("-pw", password)?);
                }
                // The protocol draws a password at each connection; this receiver prints
                // one and keeps it for the run, which is the same promise to the user.
                None => options.password = Password::Random,
            },
            "-m" => options.device_id = Some(next_word(&mut arguments, "-m")?),
            "-key" => match optional_value(&mut arguments) {
                Some(path) => key_file = Some(PathBuf::from(path)),
                None => ephemeral_seed = true,
            },
            "-reg" => {
                if let Some(path) = optional_value(&mut arguments) {
                    options.clients = ClientsSource::File(PathBuf::from(path));
                }
            }
            "-p" => match optional_value(&mut arguments) {
                // The protocol also accepts "-p tcp <list>" and "-p udp <list>"; a list is
                // refused rather than read as a port, so the message says what happened.
                Some(kind) if kind == "tcp" || kind == "udp" => {
                    return Err(Error::usage_with_usage(format!(
                        "-p {kind} <list> is not supported; this receiver takes -p or -p <base>"
                    )));
                }
                Some(base) => {
                    let base: u16 = base
                        .parse()
                        .map_err(|_| Error::usage(format!("-p: {base} is not a port")))?;
                    // The layout is a base and the two ports after it, so the last one has
                    // to stay a port.
                    base_port = Some(within("-p", base, 1, 65_533)?);
                }
                None => legacy_ports = true,
            },
            "-al" => {
                let value = next_value(&mut arguments, "-al")?
                    .parse()
                    .map_err(|_| Error::usage("-al: not a number of seconds"))?;
                options.audio_latency = within_f64("-al", value, 0.0, 10.0)?;
            }
            "-vol" => {
                let value = next_value(&mut arguments, "-vol")?
                    .parse()
                    .map_err(|_| Error::usage("-vol: not a number"))?;
                options.volume = within_f64("-vol", value, 0.0, 1.0)?;
            }
            "-db" => options.db = parse_db("-db", &next_value(&mut arguments, "-db")?)?,
            "-taper" => options.taper = true,
            "-s" => {
                let (width, height, rate) = parse_display(&next_value(&mut arguments, "-s")?)?;
                options.display.width = width;
                options.display.height = height;
                options.display.size_set = true;
                if let Some(rate) = rate {
                    options.display.refresh_rate = rate;
                }
            }
            "-fps" => {
                let value = next_value(&mut arguments, "-fps")?
                    .parse()
                    .map_err(|_| Error::usage("-fps: not a number"))?;
                options.display.max_fps = within("-fps", value, 1, 240_u16)?;
            }
            "-o" => options.display.overscanned = true,
            "-as" => options.gst.audio_sink = Some(next_word(&mut arguments, "-as")?),
            "-vs" => options.gst.video_sink = Some(next_word(&mut arguments, "-vs")?),
            "-vp" => options.gst.video_parser = Some(next_word(&mut arguments, "-vp")?),
            "-vd" => options.gst.video_decoder = Some(next_word(&mut arguments, "-vd")?),
            "-avdec" => options.gst.software_decoder = true,
            "-ca" => match optional_value(&mut arguments) {
                Some(path) => options.cover_art = Some(CoverArt::File(PathBuf::from(path))),
                // The protocol's -ca without a file is "report the art", which a terminal
                // can do: print what arrived instead of pretending to show it.
                None => options.cover_art = Some(CoverArt::Report),
            },
            "-md" => options.metadata = Some(PathBuf::from(next_word(&mut arguments, "-md")?)),
            "-dacp" => {
                options.dacp = Some(optional_value(&mut arguments).map(PathBuf::from));
            }
            "-reset" => {
                let value: f64 = next_value(&mut arguments, "-reset")?
                    .parse()
                    .map_err(|_| Error::usage("-reset: not a number of seconds"))?;
                options.reset_after = within_f64("-reset", value, 0.0, MAX_RESET)?;
            }
            "-nohold" => options.nohold = true,
            "-restrict" => options.restrict = true,
            "-allow" => {
                let value = next_word(&mut arguments, "-allow")?;
                options.allow.push(device_id("-allow", &value)?);
            }
            "-block" => {
                let value = next_word(&mut arguments, "-block")?;
                options.block.push(device_id("-block", &value)?);
            }
            "-FPSdata" => options.fps_data = true,
            "-d" => options.log_level = log::LevelFilter::Debug,
            "-v" => options.log_level = log::LevelFilter::Info,
            "-q" => options.log_level = log::LevelFilter::Error,

            // Options this receiver cannot honour are refused with the reason. A reference
            // command that asks for HLS or for recording has to fail where the user can see
            // it, not play a session that quietly is not what was asked for.
            "-hls" | "-vsync" | "-async" | "-bt709" | "-srgb" | "-fs" | "-nc" | "-nofreeze"
            | "-scrsv" | "-p2p" => {
                return Err(Error::usage_with_usage(format!(
                    "{argument} is not supported by this receiver"
                )));
            }
            "-lang" | "-slang" | "-custom" | "-rpi" | "-mp4" | "-vrtp" | "-artp" | "-vc"
            | "-v4l2" | "-f" | "-r" => {
                return Err(Error::usage_with_usage(format!(
                    "{argument} is not supported by this receiver (its feature is not implemented)"
                )));
            }

            "--key-file" => {
                key_file = Some(PathBuf::from(next_word(&mut arguments, "--key-file")?))
            }
            "--seed-file" => {
                seed_file = Some(PathBuf::from(next_word(&mut arguments, "--seed-file")?));
            }
            "--ephemeral-seed" => ephemeral_seed = true,
            "--clients-file" => {
                options.clients = ClientsSource::File(PathBuf::from(next_word(
                    &mut arguments,
                    "--clients-file",
                )?));
            }
            "--random-password" => options.password = Password::Random,
            "--password-file" => {
                password_file = Some(PathBuf::from(next_word(&mut arguments, "--password-file")?));
            }
            "--verbose" => options.log_level = log::LevelFilter::Info,
            "--quiet" => options.log_level = log::LevelFilter::Error,
            "--log-level" => {
                let level = next_word(&mut arguments, "--log-level")?;
                options.log_level = parse_level(&level)?;
            }
            "--base-port" => {
                let value = next_word(&mut arguments, "--base-port")?
                    .parse()
                    .map_err(|_| Error::usage("--base-port: not a port"))?;
                base_port = Some(within("--base-port", value, 1, 65_533_u16)?);
            }
            "--legacy-ports" => legacy_ports = true,
            "--timing-port" => {
                let value = next_word(&mut arguments, "--timing-port")?;
                overrides.timing = Some(port("--timing-port", &value)?);
            }
            "--udp-control-port" => {
                let value = next_word(&mut arguments, "--udp-control-port")?;
                overrides.control = Some(port("--udp-control-port", &value)?);
            }
            "--data-port" => {
                let value = next_word(&mut arguments, "--data-port")?;
                overrides.data = Some(port("--data-port", &value)?);
            }
            "--mirror-port" => {
                let value = next_word(&mut arguments, "--mirror-port")?;
                overrides.mirror = Some(port("--mirror-port", &value)?);
            }

            "--name" => options.name = next_word(&mut arguments, "--name")?,
            "--port" => {
                options.port = next_word(&mut arguments, "--port")?
                    .parse()
                    .map_err(|error| Error::usage(format!("--port: {error}")))?;
                port_named = true;
            }
            "--bind" => {
                let text = next_word(&mut arguments, "--bind")?;
                options.bind = Some(
                    text.parse::<IpAddr>()
                        .map_err(|_| Error::usage(format!("--bind: {text} is not an address")))?,
                );
            }
            "--no-advertise" => options.advertise = false,
            "--no-pin" => options.pin = PinMode::Disabled,
            "--pin" => {
                // The digits are optional: without them the receiver draws a one-time PIN
                // and prints it when a client asks for one.
                options.pin = match arguments.peek() {
                    Some(value) if !value.starts_with('-') => {
                        let digits: u16 = next_value(&mut arguments, "--pin")?
                            .parse()
                            .map_err(|error| Error::usage(format!("--pin: {error}")))?;
                        PinMode::Fixed(pin_option("--pin", digits)?)
                    }
                    _ => PinMode::OneTime,
                };
            }
            "--password" => {
                let value = next_word(&mut arguments, "--password")?;
                options.password = Password::Fixed(password_option("--password", value)?);
            }
            "--device-id" => options.device_id = Some(next_word(&mut arguments, "--device-id")?),
            "--seed" => {
                let text = next_word(&mut arguments, "--seed")?;
                seed_given = Some(
                    parse_hex32(&text).map_err(|error| Error::usage(format!("--seed: {error}")))?,
                );
            }
            "--log" => options.log = Some(PathBuf::from(next_word(&mut arguments, "--log")?)),
            "--render" => {
                let name = next_word(&mut arguments, "--render")?;
                options.render = match name.as_str() {
                    "null" => RenderBackend::Null,
                    "gst" => RenderBackend::Gst,
                    other => {
                        return Err(Error::usage(format!(
                            "--render: {other} is not a backend (null or gst)"
                        )));
                    }
                };
            }
            "--audio-sink" => {
                options.gst.audio_sink = Some(next_word(&mut arguments, "--audio-sink")?);
            }
            "--video-sink" => {
                options.gst.video_sink = Some(next_word(&mut arguments, "--video-sink")?);
            }
            "--video-parser" => {
                options.gst.video_parser = Some(next_word(&mut arguments, "--video-parser")?);
            }
            "--video-decoder" => {
                options.gst.video_decoder = Some(next_word(&mut arguments, "--video-decoder")?);
            }
            "--video-decoder-hevc" => {
                options.gst.video_decoder_hevc =
                    Some(next_word(&mut arguments, "--video-decoder-hevc")?);
            }
            "--warm-video" => options.gst.warm_video = true,
            "--pace-video" => options.pace_video = true,
            "--no-pace-video" => options.pace_video = false,
            "--width" => {
                let value = next_word(&mut arguments, "--width")?
                    .parse()
                    .map_err(|_| Error::usage("--width: not a number"))?;
                options.display.width = within("--width", value, 1, MAX_DISPLAY)?;
                options.display.size_set = true;
            }
            "--height" => {
                let value = next_word(&mut arguments, "--height")?
                    .parse()
                    .map_err(|_| Error::usage("--height: not a number"))?;
                options.display.height = within("--height", value, 1, MAX_DISPLAY)?;
                options.display.size_set = true;
            }
            "--refresh-rate" => {
                let value: f64 = next_word(&mut arguments, "--refresh-rate")?
                    .parse()
                    .map_err(|_| Error::usage("--refresh-rate: not a number"))?;
                options.display.refresh_rate =
                    within_f64("--refresh-rate", value, 1.0, MAX_REFRESH)?;
            }
            "--max-fps" => {
                let value = next_word(&mut arguments, "--max-fps")?
                    .parse()
                    .map_err(|_| Error::usage("--max-fps: not a number"))?;
                options.display.max_fps = within("--max-fps", value, 1, 240_u16)?;
            }
            "--overscanned" => options.display.overscanned = true,
            "--nohold" => options.nohold = true,
            "--reset" => {
                let value = next_word(&mut arguments, "--reset")?;
                let seconds: f64 = value.parse().map_err(|_| {
                    Error::usage(format!("--reset: {value} is not a number of seconds"))
                })?;
                options.reset_after = within_f64("--reset", seconds, 0.0, MAX_RESET)?;
            }
            "--restrict" => options.restrict = true,
            "--allow" => {
                let value = next_word(&mut arguments, "--allow")?;
                options.allow.push(device_id("--allow", &value)?);
            }
            "--block" => {
                let value = next_word(&mut arguments, "--block")?;
                options.block.push(device_id("--block", &value)?);
            }
            "--hevc" => options.hevc = true,
            "--features" => {
                let value = next_word(&mut arguments, "--features")?;
                let digits = value
                    .strip_prefix("0x")
                    .or_else(|| value.strip_prefix("0X"))
                    .unwrap_or(&value);
                options.features = Some(u64::from_str_radix(digits, 16).map_err(|_| {
                    Error::usage(format!("--features: {value} is not hexadecimal"))
                })?);
            }
            "--audio-latency" => {
                let value = next_word(&mut arguments, "--audio-latency")?;
                let seconds: f64 = value.parse().map_err(|_| {
                    Error::usage(format!("--audio-latency: {value} is not a number"))
                })?;
                options.audio_latency = within_f64("--audio-latency", seconds, 0.0, 10.0)?;
            }
            "--taper" => options.taper = true,
            "--fps-data" => options.fps_data = true,
            "--cover-art" => {
                // The path is optional: the protocol's -ca without one reports the art.
                options.cover_art = match arguments.peek() {
                    Some(next) if !next.starts_with('-') => {
                        let path = PathBuf::from(arguments.next().expect("the word was peeked"));
                        Some(CoverArt::File(path))
                    }
                    _ => Some(CoverArt::Report),
                };
            }
            "--metadata" => {
                options.metadata = Some(PathBuf::from(next_word(&mut arguments, "--metadata")?));
            }
            "--dacp" => {
                options.dacp = Some(Some(PathBuf::from(next_word(&mut arguments, "--dacp")?)));
            }
            "--db" => options.db = parse_db("--db", &next_word(&mut arguments, "--db")?)?,
            "--volume" => {
                let value = next_word(&mut arguments, "--volume")?;
                let level: f64 = value
                    .parse()
                    .map_err(|_| Error::usage(format!("--volume: {value} is not a number")))?;
                options.volume = within_f64("--volume", level, 0.0, 1.0)?;
            }
            "--audio-codec" => {
                let name = next_word(&mut arguments, "--audio-codec")?;
                options.audio_codec = match name.as_str() {
                    "alac" => AudioCodec::Alac,
                    "aac-eld" => AudioCodec::AacEld,
                    other => {
                        return Err(Error::usage(format!(
                            "--audio-codec: {other} is not a coding (alac or aac-eld)"
                        )));
                    }
                };
            }
            "--data-dir" => {
                options.data_dir = Some(PathBuf::from(next_word(&mut arguments, "--data-dir")?));
            }
            other => {
                return Err(Error::usage_with_usage(format!("unknown option {other}")));
            }
        }
    }

    // The identity has four sources and exactly one may be given: two of them is a command
    // line that says two things, and guessing which one wins is how a user ends up with an
    // identity they did not expect.
    let sources = [
        seed_given.is_some(),
        seed_file.is_some(),
        key_file.is_some(),
        ephemeral_seed,
    ]
    .into_iter()
    .filter(|set| *set)
    .count();
    if sources > 1 {
        return Err(Error::usage(
            "--seed, --seed-file, -key <file> and -key/--ephemeral-seed say which identity to use; give one",
        ));
    }
    options.identity = if let Some(seed) = seed_given {
        Identity::Given(seed)
    } else if let Some(path) = seed_file {
        Identity::File(path)
    } else if let Some(path) = key_file {
        Identity::File(path)
    } else if ephemeral_seed {
        Identity::Ephemeral
    } else {
        Identity::Default
    };
    // A run that keeps no identity has nowhere to remember a client either, unless the
    // caller named a registry: -key without a file promises nothing is left behind.
    if options.identity == Identity::Ephemeral && options.clients == ClientsSource::Default {
        options.clients = ClientsSource::Memory;
    }

    if let Some(path) = password_file {
        if !matches!(options.password, Password::None) {
            return Err(Error::usage(
                "--password-file and -pw/--password/--random-password both set the password; give one",
            ));
        }
        options.password = Password::FromFile(path);
    }

    if options.hevc && options.features.is_some() {
        return Err(Error::usage(
            "-h265 and --features both set the advertised features; --features already decides bit 42",
        ));
    }

    if base_port.is_some() && port_named {
        return Err(Error::usage(
            "-p <base> sets the whole port layout, so it cannot be combined with --port",
        ));
    }
    if base_port.is_some() && overrides.any() {
        return Err(Error::usage(
            "-p <base> sets the whole port layout, so it cannot be combined with --timing-port, --udp-control-port, --data-port or --mirror-port",
        ));
    }
    options.ports = match (base_port, legacy_ports || overrides.any()) {
        (Some(base), _) => Ports::Base(base),
        (None, true) => Ports::Legacy(overrides),
        (None, false) => Ports::Default,
    };

    // -restrict is what makes -allow mean anything: the library ignores the list without
    // it, so a receiver started with only -allow admits everybody.
    if !options.allow.is_empty() && !options.restrict {
        options.restrict = true;
        options
            .warnings
            .push("-allow implies -restrict: only the listed devices may connect".to_owned());
    }
    if options.restrict && options.allow.is_empty() {
        options
            .warnings
            .push("-restrict with no -allow refuses every device".to_owned());
    }
    options.allow.sort_unstable();
    options.allow.dedup();
    options.block.sort_unstable();
    options.block.dedup();

    Ok(Parsed::Run(Box::new(options)))
}

/// The next word, whatever it looks like.
///
/// Numeric options keep this: a decibel range of "-30:0" is one value that happens to
/// start with a dash.
fn next_value(
    arguments: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    option: &str,
) -> Result<String, Error> {
    arguments
        .next()
        .ok_or_else(|| Error::usage(format!("{option} needs a value")))
}

/// The next word, when a value may not be mistaken for another option.
///
/// Names, paths and device identifiers use this: without it, "parlor -n -p 7000" silently
/// names the receiver "-p" and drops the port.
fn next_word(
    arguments: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    option: &str,
) -> Result<String, Error> {
    match arguments.peek() {
        Some(next) if next.starts_with('-') => Err(Error::usage(format!(
            "{option} needs a value; {next} is the next option"
        ))),
        _ => next_value(arguments, option),
    }
}

/// Takes the next argument when it is a value rather than another option.
///
/// The protocol's optional-value options (-pin, -pw, -m, -key, -ca, -dacp) are written
/// "-pin 1234" or "-pin", and the difference is whether the next word starts with a dash.
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
) -> Result<T, Error> {
    if value < low || value > high {
        return Err(Error::usage(format!(
            "{option}: {value} is outside {low}..={high}"
        )));
    }
    Ok(value)
}

/// Checks a floating-point option is a real number inside its range.
///
/// A NaN compares false against both ends of a range, so a plain range check lets it
/// through and the session then runs with a value nobody asked for.
fn within_f64(option: &str, value: f64, low: f64, high: f64) -> Result<f64, Error> {
    if !value.is_finite() || value < low || value > high {
        return Err(Error::usage(format!(
            "{option}: {value} is outside {low}..={high}"
        )));
    }
    Ok(value)
}

/// Checks a PIN is one a person can type: four digits, 0001 to 9999.
///
/// The protocol refuses anything outside that range when it parses the option, and a
/// fixed PIN outside it is a receiver no client can pair with.
fn pin_option(option: &str, digits: u16) -> Result<u16, Error> {
    if !(1..=9999).contains(&digits) {
        return Err(Error::usage(format!(
            "{option}: {digits} is not a PIN (0001 to 9999)"
        )));
    }
    Ok(digits)
}

/// Checks a password against the minimum length the protocol enforces.
fn password_option(option: &str, value: String) -> Result<String, Error> {
    if value.chars().count() < 4 {
        return Err(Error::usage(format!(
            "{option}: a password needs at least four characters"
        )));
    }
    Ok(value)
}

/// Reads a log level by name.
fn parse_level(name: &str) -> Result<log::LevelFilter, Error> {
    Ok(match name {
        "off" => log::LevelFilter::Off,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        other => {
            return Err(Error::usage(format!(
                "--log-level: {other} is not a level (off, error, warn, info, debug, trace)"
            )));
        }
    })
}

/// Reads a display the way the protocol writes one: a width, an x, a height, and an
/// optional refresh rate after an at sign.
fn parse_display(text: &str) -> Result<(u64, u64, Option<f64>), Error> {
    let (size, rate) = match text.split_once('@') {
        Some((size, rate)) => {
            let rate: f64 = rate
                .parse()
                .map_err(|_| Error::usage(format!("-s: {rate} is not a refresh rate")))?;
            (size, Some(rate))
        }
        None => (text, None),
    };
    let (width, height) = size
        .split_once(['x', 'X'])
        .ok_or_else(|| Error::usage(format!("-s: {size} is not a width and a height")))?;
    let width: u64 = width
        .parse()
        .map_err(|_| Error::usage(format!("-s: {width} is not a width")))?;
    let height: u64 = height
        .parse()
        .map_err(|_| Error::usage(format!("-s: {height} is not a height")))?;
    if width == 0 || height == 0 {
        return Err(Error::usage(format!("-s: {text} is not a usable display")));
    }
    let width = within("-s", width, 1, MAX_DISPLAY)?;
    let height = within("-s", height, 1, MAX_DISPLAY)?;
    let rate = match rate {
        Some(rate) => Some(within_f64("-s", rate, 1.0, MAX_REFRESH)?),
        None => None,
    };
    Ok((width, height, rate))
}

/// Reads a low-to-high decibel range.
fn parse_db(option: &str, text: &str) -> Result<(f64, f64), Error> {
    let (low, high) = text
        .split_once(':')
        .ok_or_else(|| Error::usage(format!("{option}: {text} is not low:high")))?;
    let low: f64 = low
        .parse()
        .map_err(|_| Error::usage(format!("{option}: {low} is not a number")))?;
    let high: f64 = high
        .parse()
        .map_err(|_| Error::usage(format!("{option}: {high} is not a number")))?;
    if !low.is_finite() || !high.is_finite() || low >= high {
        return Err(Error::usage(format!(
            "{option}: the range must be low:high with low below high, got {text}"
        )));
    }
    Ok((low, high))
}

/// Reads a device identifier the way a client writes one: twelve hex digits.
///
/// The library compares the list against what a client states in SETUP as one exact
/// string, so a colon-separated address here would match nothing and refuse every device.
/// Normalising once, where it is read, is what keeps "-allow AA:BB:.." and
/// "-allow aabbccddeeff" the same device.
fn device_id(option: &str, text: &str) -> Result<String, Error> {
    let digits: String = text
        .chars()
        .filter(|character| !matches!(character, ':' | '-' | '.'))
        .collect();
    if digits.len() != 12
        || !digits
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(Error::usage(format!(
            "{option}: {text} is not a device address (twelve hex digits)"
        )));
    }
    Ok(digits.to_ascii_uppercase())
}

/// Reads an explicit port number.
fn port(option: &str, text: &str) -> Result<u16, Error> {
    let port: u16 = text
        .parse()
        .map_err(|_| Error::usage(format!("{option}: {text} is not a port")))?;
    within(option, port, 1, 65_535)
}

#[cfg(test)]
mod tests {
    use super::{
        ClientsSource, CoverArt, GstOptions, Identity, MAX_RESET, Options, Parsed, Password, Ports,
        RenderBackend, parse_arguments,
    };
    use rust_airplay::server::PinMode;

    fn parse(arguments: &[&str]) -> Result<Parsed, super::Error> {
        parse_arguments(arguments.iter().map(|argument| (*argument).to_owned()))
    }

    fn options(arguments: &[&str]) -> Options {
        match parse(arguments).expect("the command line parses") {
            Parsed::Run(options) => *options,
            other => panic!("expected a run, got {other:?}"),
        }
    }

    fn refused(arguments: &[&str]) -> String {
        match parse(arguments) {
            Ok(parsed) => panic!("{arguments:?} was accepted as {parsed:?}"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn a_compatible_command_line_reaches_the_same_options() {
        // The point of the compatibility names: a command written for the reference runs here.
        let options = options(&[
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
        ]);
        assert_eq!(options.name, "Parlor");
        assert_eq!(options.pin, PinMode::Fixed(1234));
        assert_eq!((options.display.width, options.display.height), (1280, 720));
        assert_eq!(options.display.refresh_rate, 30.0);
        assert_eq!(options.display.max_fps, 24);
        assert!(options.display.overscanned);
        assert_eq!(options.audio_latency, 0.5);
        assert_eq!(options.volume, 0.8);
        assert!(options.taper);
        assert_eq!(options.db, (-20.0, 0.0));
        assert!(options.nohold);
        assert!(options.restrict);
        assert_eq!(options.allow, vec!["AABBCCDDEEFF".to_owned()]);
        assert!(options.fps_data);
    }

    #[test]
    fn the_port_option_keeps_the_layout() {
        // -p alone is the fixed layout; -p n names the whole layout from a base port.
        assert_eq!(options(&["-p"]).ports, Ports::Legacy(Default::default()));
        assert_eq!(options(&["-p", "7000"]).ports, Ports::Base(7000));
        assert_eq!(
            options(&["--data-port", "6005"]).ports,
            Ports::Legacy(super::PortOverrides {
                data: Some(6005),
                ..super::PortOverrides::default()
            })
        );
    }

    #[test]
    fn key_and_password_take_an_optional_value() {
        // -key with no file means the identity is not stored; -pw with no password means a
        // one is drawn and printed.
        let ephemeral = options(&["-key"]);
        assert_eq!(ephemeral.identity, Identity::Ephemeral);
        // Nothing is stored, so the registry is not stored either.
        assert_eq!(ephemeral.clients, ClientsSource::Memory);

        let stored = options(&["-key", "seed.txt"]);
        assert_eq!(stored.identity, Identity::File("seed.txt".into()));
        assert_eq!(stored.clients, ClientsSource::Default);

        assert_eq!(options(&["-pw"]).password, Password::Random);
        assert_eq!(
            options(&["-pw", "hunter2"]).password,
            Password::Fixed("hunter2".to_owned())
        );
    }

    #[test]
    fn cover_art_with_no_file_reports_it() {
        // The protocol's -ca without a path reports the art; a receiver that ignored it
        // would be a receiver whose documentation is wrong.
        assert_eq!(options(&["-ca"]).cover_art, Some(CoverArt::Report));
        assert_eq!(
            options(&["-ca", "art.png"]).cover_art,
            Some(CoverArt::File("art.png".into()))
        );
        assert_eq!(options(&["--cover-art"]).cover_art, Some(CoverArt::Report));
    }

    #[test]
    fn a_remote_control_path_is_optional() {
        assert_eq!(options(&["-dacp"]).dacp, Some(None));
        assert_eq!(
            options(&["-dacp", "rc.txt"]).dacp,
            Some(Some("rc.txt".into()))
        );
    }

    #[test]
    fn the_new_switches_are_read() {
        let options = options(&[
            "--bind",
            "127.0.0.1",
            "--no-advertise",
            "--data-dir",
            "here",
        ]);
        assert_eq!(options.bind, Some("127.0.0.1".parse().expect("an address")));
        assert!(!options.advertise);
        assert_eq!(options.data_dir, Some("here".into()));
    }

    #[test]
    fn version_and_help_are_not_runs() {
        assert_eq!(parse(&["--version"]).expect("parsed"), Parsed::Version);
        assert_eq!(parse(&["-V"]).expect("parsed"), Parsed::Version);
        assert_eq!(parse(&["--help"]).expect("parsed"), Parsed::Help);
        assert_eq!(parse(&["-h"]).expect("parsed"), Parsed::Help);
    }

    #[test]
    fn an_option_this_receiver_cannot_honour_is_refused_with_a_reason() {
        assert!(refused(&["-hls"]).contains("not supported"));
        assert!(refused(&["-mp4", "out.mp4"]).contains("not supported"));
    }

    #[test]
    fn options_outside_their_range_are_refused() {
        // Each of these was accepted before the audit and either clamped silently or
        // produced a value the protocol cannot use.
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
            let error = refused(arguments);
            assert!(!error.is_empty(), "{arguments:?}");
        }
    }

    #[test]
    fn a_nan_is_not_a_number_the_receiver_can_use() {
        // A NaN compares false against both ends of a range, so it used to pass every
        // bounds check and then quietly became full volume or a zero delay.
        for arguments in [
            &["-vol", "NaN"][..],
            &["-vol", "nan"],
            &["--volume", "NaN"],
            &["-al", "NaN"],
            &["--audio-latency", "NaN"],
            &["-al", "inf"],
            &["--audio-latency", "infinity"],
        ] {
            refused(arguments);
        }
        // The boundary itself is still a number the receiver can use.
        assert_eq!(options(&["-vol", "1"]).volume, 1.0);
        assert_eq!(options(&["-al", "10"]).audio_latency, 10.0);
    }

    #[test]
    fn a_display_is_bounded_like_every_other_size() {
        // -s used to accept any size at all while --width refused anyone over 16384.
        refused(&["-s", "999999x999999"]);
        refused(&["-s", "16385x1080"]);
        refused(&["-s", "1920x1080@0"]);
        refused(&["-s", "1920x1080@1000"]);
        let options = options(&["-s", "16384x16384@240"]);
        assert_eq!(
            (options.display.width, options.display.height),
            (16384, 16384)
        );
        assert_eq!(options.display.refresh_rate, 240.0);
    }

    #[test]
    fn a_feedback_timeout_that_cannot_be_a_duration_is_refused() {
        // A finite float that is too large for a Duration used to reach it and panic.
        refused(&["-reset", "1e20"]);
        refused(&["--reset", "1e20"]);
        refused(&["-reset", "NaN"]);
        let longest = options(&["-reset", &MAX_RESET.to_string()]);
        assert_eq!(longest.reset_after, MAX_RESET);
        assert_eq!(options(&["-reset", "0"]).reset_after, 0.0);
    }

    #[test]
    fn a_seed_that_is_not_hexadecimal_is_refused_rather_than_sliced_apart() {
        // Sixty four bytes whose second byte is inside a multi-byte character used to panic
        // on a slice boundary instead of reporting a bad argument.
        let text = format!("a{}a", "\u{e9}".repeat(31));
        let error = refused(&["--seed", &text]);
        assert!(
            error.contains("ASCII") || error.contains("hexadecimal"),
            "{error}"
        );
        assert_eq!(
            options(&["--seed", &"2a".repeat(32)]).identity,
            Identity::Given([0x2a; 32])
        );
    }

    #[test]
    fn a_value_may_not_be_another_option() {
        // Every one of these used to swallow the option after it: -n -p named the receiver
        // "-p" and dropped the port, and -allow -restrict quietly started an unrestricted
        // receiver.
        for arguments in [
            &["-n", "-p", "7000"][..],
            &["--name", "--quiet"],
            &["-allow", "-restrict"],
            &["--block", "--quiet"],
            &["-md", "-q"],
            &["--log", "--quiet"],
            &["--data-dir", "-q"],
        ] {
            let error = refused(arguments);
            assert!(error.contains("needs a value"), "{arguments:?}: {error}");
        }
        // A numeric option keeps its value even when it starts with a dash.
        assert_eq!(options(&["-db", "-30:0"]).db, (-30.0, 0.0));
    }

    #[test]
    fn two_settings_for_one_thing_are_refused() {
        refused(&["-h265", "--features", "10"]);
        refused(&["--seed", &"2a".repeat(32), "-key", "seed.txt"]);
        refused(&["-pw", "abcdef", "--password-file", "pw.txt"]);
        refused(&["-p", "7000", "--port", "7100"]);
        refused(&["-p", "7000", "--data-port", "6005"]);
    }

    #[test]
    fn an_allow_list_is_normalised_and_implies_restriction() {
        // The library compares one exact string, so "aa:bb:cc:dd:ee:ff" and
        // "AABBCCDDEEFF" have to be the same device or the list refuses everything.
        let options = options(&[
            "-allow",
            "aa:bb:cc:dd:ee:ff",
            "-allow",
            "AABBCCDDEEFF",
            "-block",
            "11-22-33-44-55-66",
        ]);
        assert!(options.restrict);
        assert_eq!(options.allow, vec!["AABBCCDDEEFF".to_owned()]);
        assert_eq!(options.block, vec!["112233445566".to_owned()]);
        assert!(
            options
                .warnings
                .iter()
                .any(|warning| warning.contains("-restrict")),
            "{:?}",
            options.warnings
        );
        refused(&["-allow", "aa:bb"]);
        refused(&["-allow", "nothexadecimal"]);
    }

    #[test]
    fn values_inside_their_range_still_parse() {
        // The boundaries are where a valid value stops being valid, not one short of it.
        let options = options(&[
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
        ]);
        assert_eq!(options.pin, PinMode::Fixed(9999));
        assert_eq!(options.volume, 1.0);
        assert_eq!(options.audio_latency, 10.0);
        assert_eq!(options.ports, Ports::Base(65_533));
    }

    #[test]
    fn the_log_level_is_a_setting_and_warnings_are_the_default() {
        // The device run showed that a receiver printing every media record is unreadable:
        // the session's own lines are opt-in.
        assert_eq!(Options::default().log_level, log::LevelFilter::Warn);
        assert_eq!(options(&["-v"]).log_level, log::LevelFilter::Info);
        assert_eq!(options(&["-d"]).log_level, log::LevelFilter::Debug);
        assert_eq!(options(&["-q"]).log_level, log::LevelFilter::Error);
        assert_eq!(
            options(&["--log-level", "trace"]).log_level,
            log::LevelFilter::Trace
        );
        refused(&["--log-level", "loud"]);
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

    #[test]
    fn the_picture_is_not_paced_unless_asked_for() {
        // Off is the shortest path from device to screen; --pace-video spends a little
        // latency to smooth a stream that arrives in bursts.
        assert!(!Options::default().pace_video);
        assert!(options(&["--pace-video"]).pace_video);
        assert!(!options(&["--no-pace-video"]).pace_video);
    }

    #[test]
    fn the_gstreamer_settings_are_collected() {
        let options = options(&[
            "-as",
            "alsasink",
            "-vs",
            "ximagesink",
            "-vp",
            "h264parse",
            "-vd",
            "avdec_h264",
            "-avdec",
            "--warm-video",
        ]);
        assert_eq!(
            options.gst,
            GstOptions {
                audio_sink: Some("alsasink".to_owned()),
                video_sink: Some("ximagesink".to_owned()),
                video_parser: Some("h264parse".to_owned()),
                video_decoder: Some("avdec_h264".to_owned()),
                video_decoder_hevc: None,
                software_decoder: true,
                warm_video: true,
            }
        );
    }
}
