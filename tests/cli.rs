// SPDX-License-Identifier: GPL-3.0-or-later
//! What the built program does with a command line.
//!
//! The unit tests cover the parser; these cover the program: the exit code a script
//! branches on, the text a person reads, and the two things that used to stop a receiver
//! by crashing it.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The program under test.
fn parlor() -> Command {
    Command::new(env!("CARGO_BIN_EXE_parlor"))
}

/// Runs the program with these arguments and waits for it.
fn run(arguments: &[&str]) -> (i32, String, String) {
    let output = parlor().args(arguments).output().expect("the program runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn help_is_a_success_and_lists_every_long_option() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0);
    for option in [
        "--pin",
        "--no-pin",
        "--quiet",
        "--verbose",
        "--version",
        "--data-dir",
        "--bind",
        "--password-file",
        "--seed-file",
    ] {
        assert!(stdout.contains(option), "{option} is not in the usage text");
    }
}

#[test]
fn the_version_is_a_success() {
    let (code, stdout, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "{stdout}");
    assert_eq!(run(&["-V"]).0, 0);
}

#[test]
fn a_command_line_this_program_cannot_accept_exits_two() {
    for arguments in [
        &["--no-such-option"][..],
        &["-n"],
        &["-n", "-p", "7000"],
        &["-allow", "-restrict"],
        &["-vol", "NaN"],
        &["-al", "NaN"],
        &["-s", "99999x99999"],
        &["-reset", "1e20"],
        &["--password-file", "pw", "-pw", "abcdef"],
    ] {
        let (code, _, stderr) = run(arguments);
        assert_eq!(code, 2, "{arguments:?} should be a usage error: {stderr}");
        assert!(
            !stderr.contains("panicked"),
            "{arguments:?} panicked: {stderr}"
        );
        assert!(!stderr.is_empty(), "{arguments:?} said nothing");
    }
}

#[test]
fn a_seed_that_is_not_hexadecimal_reports_instead_of_panicking() {
    // Sixty four bytes whose second byte is inside a multi-byte character: this used to
    // stop a receiver with a panic and exit 101.
    let seed = format!("a{}a", "\u{e9}".repeat(31));
    assert_eq!(seed.len(), 64);
    let (code, _, stderr) = run(&["--seed", &seed]);
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(code, 2, "{stderr}");
}

#[test]
fn a_closed_output_pipe_does_not_stop_the_program() {
    // A receiver whose output is piped into something that exits first must stop quietly,
    // not panic on the first write.
    let mut child = parlor()
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the program starts");
    // Dropping the read end closes the pipe before the child has finished writing.
    drop(child.stdout.take());
    let status = child.wait().expect("the program finishes");
    assert_eq!(status.code(), Some(0));
}

#[cfg(not(feature = "gst"))]
#[test]
fn asking_for_a_backend_this_build_lacks_is_a_run_error() {
    let (code, _, stderr) = run(&["--render", "gst"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("gst feature"), "{stderr}");
}

#[test]
fn a_receiver_starts_and_keeps_its_identity() {
    // A whole run, with the ports left to the system and mDNS off so a test does not
    // announce itself on the network: the identity file is the visible result.
    let directory = std::env::temp_dir().join(format!("parlor-cli-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the test directory is created");

    let mut child = parlor()
        .args([
            "--render",
            "null",
            "--port",
            "0",
            "--no-advertise",
            "--data-dir",
            directory.to_str().expect("a path"),
            "-q",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the program starts");

    let seed = directory.join("identity.seed");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !seed.exists() {
        std::thread::sleep(Duration::from_millis(50));
    }
    let running = child.try_wait().expect("the child is readable").is_none();
    let _ = child.kill();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let _ = child.wait();

    assert!(running, "the receiver stopped early: {stderr}");
    let text = std::fs::read_to_string(&seed).expect("the identity is written");
    assert_eq!(text.trim().len(), 64, "{text}");
    assert!(
        text.trim()
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
    let _ = std::fs::remove_dir_all(&directory);
}
