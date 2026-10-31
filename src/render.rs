// SPDX-License-Identifier: GPL-3.0-or-later
//! Which backend plays the media.
//!
//! The renderer is chosen before anything is advertised, because a backend this build or
//! this machine cannot provide is something to say now rather than when a client is
//! already waiting for sound.

use std::sync::Arc;

use rust_airplay::{NullRenderer, Renderer};

use crate::Error;
use crate::cli::{Options, RenderBackend};

/// Builds the renderer a command line asked for.
///
/// # Errors
///
/// Returns a message when GStreamer was asked for and this build does not have it, or
/// when the launcher or the pipeline cannot be found.
pub fn build(options: &Options) -> Result<Arc<dyn Renderer>, Error> {
    // Settings that only GStreamer reads are worth a line when they cannot do anything:
    // a user who passed --audio-sink and hears nothing should not have to read the source
    // to find out why.
    if options.render == RenderBackend::Null && options.gst.any() {
        log::warn!("--render null plays nothing, so the GStreamer settings have no effect");
    }
    match options.render {
        RenderBackend::Null => Ok(Arc::new(NullRenderer::new())),
        RenderBackend::Gst => open_gst(options),
    }
}

/// The GStreamer backend, when this build has it.
#[cfg(feature = "gst")]
fn open_gst(options: &Options) -> Result<Arc<dyn Renderer>, Error> {
    use rust_airplay::VideoCodec;

    // The defaults come from the environment, and a flag overrides one of them.
    let mut backend = rust_airplay_gstream::Options::default();
    if let Some(sink) = &options.gst.audio_sink {
        backend = backend.with_audio_sink(sink.clone());
    }
    if let Some(sink) = &options.gst.video_sink {
        backend = backend.with_video_sink(sink.clone());
    }
    if let Some(parser) = &options.gst.video_parser {
        backend = backend.with_video_parser(VideoCodec::H264, parser.clone());
    }
    if let Some(decoder) = &options.gst.video_decoder {
        backend = backend.with_video_decoder(VideoCodec::H264, decoder.clone());
    }
    if let Some(decoder) = &options.gst.video_decoder_hevc {
        backend = backend.with_video_decoder(VideoCodec::H265, decoder.clone());
    }
    if options.gst.software_decoder {
        // The software decoder is the one that always exists; a machine without a hardware
        // decoder needs it, and a user debugging a picture wants to know which one ran.
        backend = backend
            .with_video_decoder(VideoCodec::H264, "avdec_h264")
            .with_video_decoder(VideoCodec::H265, "avdec_h265");
    }
    if options.gst.warm_video {
        backend = backend.with_warm_video(true);
    }
    // Since the 0.3 backend a sink and a decoder are two choices: a hardware sink fed by
    // the default avdec_* decodes every frame on the CPU, and a mirror stream that only
    // just keeps up shows as a stutter. Say which one this run will use, because the
    // difference is a flag nobody would guess.
    if options.gst.video_decoder.is_none() && !options.gst.software_decoder {
        log::warn!(
            "video is decoded in software; -vd d3d11h264dec (or the decoder this machine has) uses the GPU"
        );
    }

    let renderer = rust_airplay_gstream::GstRenderer::with_options(backend)
        .map_err(|error| Error::run(format!("--render gst: {error}")))?;
    // What a stream will actually be built from, because a hardware sink says nothing about
    // the decoder behind it and the two are chosen separately since 0.3.
    log::info!(
        "render    h264   {}",
        renderer.video_description(VideoCodec::H264)
    );
    log::info!(
        "render    h265   {}",
        renderer.video_description(VideoCodec::H265)
    );
    log::info!(
        "render    gst    audio {}, video {}",
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
            "cold (started when a client connects; --warm-video starts it now)"
        }
    );
    Ok(Arc::new(renderer))
}

/// The GStreamer backend, which this build does not have.
#[cfg(not(feature = "gst"))]
fn open_gst(options: &Options) -> Result<Arc<dyn Renderer>, Error> {
    let _ = options;
    Err(Error::run(
        "--render gst needs a build with the gst feature: cargo run --features gst --",
    ))
}
