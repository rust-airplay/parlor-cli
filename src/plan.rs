// SPDX-License-Identifier: GPL-3.0-or-later
//! What a command line means to the receiver, derived once and tested without I/O.
//!
//! The parser answers "what did the user write"; this module answers "what does that
//! mean": which ports to bind, which display to advertise, which feature bitmap, how a
//! slider position becomes a gain. Nothing here opens a socket or a file.

use std::time::Duration;

use rust_airplay::media::ports::MediaPorts;
use rust_airplay::server::{ClientPolicy, Display};
use rust_airplay::{Volume, VolumeCurve};

use crate::Error;
use crate::cli::{DisplayOptions, Options, PortOverrides, Ports};

/// The feature bitmap bit that allows HEVC video.
///
/// The protocol's -h265 is the default bitmap with this bit set; the number is stated by
/// the protocol and by the library's own documentation of the features field.
const HEVC_FEATURE: u64 = 1 << 42;

/// The width -h265 offers when the caller did not state a size.
const HEVC_WIDTH: u64 = 3840;

/// The height -h265 offers when the caller did not state a size.
const HEVC_HEIGHT: u64 = 2160;

/// Everything a run derives from its command line before it starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// The port the control service binds.
    pub control_port: u16,
    /// The ports the media session binds.
    pub ports: MediaPorts,
    /// The display offered in GET /info.
    pub display: Display,
    /// The feature bitmap to advertise.
    pub features: Option<u64>,
    /// Which clients may start a session.
    pub client_policy: ClientPolicy,
    /// The volume a session starts at.
    pub volume: Volume,
    /// How a slider position becomes a gain.
    pub volume_curve: VolumeCurve,
    /// The audio delay reported to a client.
    pub audio_latency: Duration,
    /// How long a session may go without feedback.
    pub feedback_timeout: Option<Duration>,
}

impl Plan {
    /// Derives what the receiver needs from a parsed command line.
    ///
    /// # Errors
    ///
    /// Returns [Error::Usage] when a value cannot be a duration; the parser accepts only
    /// finite values, so this is a guard rather than an expected path.
    pub fn derive(options: &Options) -> Result<Self, Error> {
        let (control_port, ports) = layout(options);
        Ok(Self {
            control_port,
            ports,
            display: display(&options.display, options.hevc),
            features: features(options),
            client_policy: ClientPolicy {
                restrict: options.restrict,
                allow: options.allow.clone(),
                block: options.block.clone(),
            },
            volume: Volume::new(options.volume),
            volume_curve: VolumeCurve {
                taper: options.taper,
                low_db: options.db.0,
                high_db: options.db.1,
            },
            audio_latency: seconds(options.audio_latency, "audio latency")?,
            feedback_timeout: if options.reset_after > 0.0 {
                Some(seconds(options.reset_after, "feedback timeout")?)
            } else {
                None
            },
        })
    }
}

/// The control port and the media ports a command line asks for.
fn layout(options: &Options) -> (u16, MediaPorts) {
    match options.ports {
        // A base port names the whole layout: the protocol's udp[0..2] are the base and the
        // two ports after it, and the control port is the base plus one.
        Ports::Base(base) => {
            let ports = MediaPorts {
                timing: base,
                control: base.saturating_add(1),
                data: base.saturating_add(2),
                mirror: base,
            };
            (base.saturating_add(1), ports)
        }
        // Everything else keeps the control port the caller asked for and moves only the
        // media ports off their layout.
        Ports::Legacy(overrides) => (options.port, overridden(MediaPorts::legacy(), overrides)),
        Ports::Default => (options.port, MediaPorts::automatic()),
    }
}

/// Moves the media ports the caller named.
fn overridden(mut ports: MediaPorts, overrides: PortOverrides) -> MediaPorts {
    if let Some(port) = overrides.timing {
        ports.timing = port;
    }
    if let Some(port) = overrides.control {
        ports.control = port;
    }
    if let Some(port) = overrides.data {
        ports.data = port;
    }
    if let Some(port) = overrides.mirror {
        ports.mirror = port;
    }
    ports
}

/// The display offered to a client.
///
/// -h265 is one bit and a bigger default display, and the bigger display is only a
/// default: a caller who stated a size with -s means it, and asking for HEVC at 1080p is
/// a thing a receiver on a small screen may want.
fn display(display: &DisplayOptions, hevc: bool) -> Display {
    let enlarge = hevc && !display.size_set;
    Display {
        width: if enlarge { HEVC_WIDTH } else { display.width },
        height: if enlarge { HEVC_HEIGHT } else { display.height },
        refresh_rate: display.refresh_rate,
        max_fps: u64::from(display.max_fps),
        overscanned: display.overscanned,
    }
}

/// The feature bitmap to advertise, when the caller asked for one.
fn features(options: &Options) -> Option<u64> {
    options.features.or_else(|| {
        options
            .hevc
            .then_some(rust_airplay::server::DEFAULT_FEATURES | HEVC_FEATURE)
    })
}

/// A finite number of seconds as a duration.
fn seconds(value: f64, what: &str) -> Result<Duration, Error> {
    Duration::try_from_secs_f64(value)
        .map_err(|_| Error::usage(format!("{what}: {value} is not a length of time")))
}

#[cfg(test)]
mod tests {
    use super::{HEVC_FEATURE, Plan, layout};
    use crate::cli::{DisplayOptions, Options, PortOverrides, Ports};

    fn options() -> Options {
        Options::default()
    }

    #[test]
    fn a_base_port_names_the_whole_layout() {
        let options = Options {
            ports: Ports::Base(7000),
            ..options()
        };
        let (control, ports) = layout(&options);
        assert_eq!(control, 7001);
        assert_eq!(
            (ports.timing, ports.control, ports.data, ports.mirror),
            (7000, 7001, 7002, 7000)
        );
    }

    #[test]
    fn the_fixed_layout_keeps_the_control_port() {
        let options = Options {
            port: 7100,
            ports: Ports::Legacy(PortOverrides::default()),
            ..options()
        };
        let (control, ports) = layout(&options);
        assert_eq!(control, 7100);
        assert_eq!(
            (ports.timing, ports.control, ports.data, ports.mirror),
            (7011, 6001, 6000, 7100)
        );
    }

    #[test]
    fn a_named_port_moves_only_that_one() {
        let options = Options {
            ports: Ports::Legacy(PortOverrides {
                data: Some(6005),
                ..PortOverrides::default()
            }),
            ..options()
        };
        let (_, ports) = layout(&options);
        assert_eq!(ports.data, 6005);
        assert_eq!(ports.timing, 7011);
    }

    #[test]
    fn hevc_offers_four_k_only_when_no_size_was_stated() {
        let plan = Plan::derive(&options()).expect("derived");
        assert_eq!((plan.display.width, plan.display.height), (1920, 1080));
        assert_eq!(plan.features, None);

        let hevc = Options {
            hevc: true,
            ..options()
        };
        let plan = Plan::derive(&hevc).expect("derived");
        assert_eq!((plan.display.width, plan.display.height), (3840, 2160));
        assert_eq!(
            plan.features,
            Some(rust_airplay::server::DEFAULT_FEATURES | HEVC_FEATURE)
        );

        // A stated size wins: HEVC at 1080p is a question a caller may ask.
        let stated = Options {
            hevc: true,
            display: DisplayOptions {
                width: 1280,
                height: 720,
                size_set: true,
                ..DisplayOptions::default()
            },
            ..options()
        };
        let plan = Plan::derive(&stated).expect("derived");
        assert_eq!((plan.display.width, plan.display.height), (1280, 720));
    }

    #[test]
    fn the_volume_curve_and_the_delay_are_carried_through() {
        let options = Options {
            volume: 0.5,
            taper: true,
            db: (-40.0, -3.0),
            audio_latency: 0.5,
            reset_after: 30.0,
            ..options()
        };
        let plan = Plan::derive(&options).expect("derived");
        assert!(plan.volume_curve.taper);
        assert_eq!(
            (plan.volume_curve.low_db, plan.volume_curve.high_db),
            (-40.0, -3.0)
        );
        assert_eq!(plan.audio_latency, std::time::Duration::from_millis(500));
        assert_eq!(
            plan.feedback_timeout,
            Some(std::time::Duration::from_secs(30))
        );
        assert_eq!(plan.volume, rust_airplay::Volume::new(0.5));
    }
}
