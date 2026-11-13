// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use std::{path::Path, process::ExitCode};

const USAGE: &str = "usage: cargo xtask policy\n       cargo xtask verify [--fuzz-seconds N]   (N = 0 skips fuzzing; default 60)";

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["policy"] => match xtask::policy(root) {
            Ok(()) => {
                println!("policy: all checks passed");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("policy: FAIL: {e}");
                ExitCode::FAILURE
            }
        },
        ["ffi-header"] => finish(
            xtask::ffi::header(root, false)
                .map_err(|e| eprintln!("{e}"))
                .is_ok(),
        ),
        ["ffi-header", "--write"] => finish(
            xtask::ffi::header(root, true)
                .map_err(|e| eprintln!("{e}"))
                .is_ok(),
        ),
        ["ffi-c-compile"] => finish(
            xtask::ffi::c_compile(root, false)
                .map_err(|e| eprintln!("{e}"))
                .is_ok(),
        ),
        ["ffi-c-run"] => finish(
            xtask::ffi::c_run(root)
                .map_err(|e| eprintln!("{e}"))
                .is_ok(),
        ),
        ["ffi-cxx"] => finish(
            xtask::ffi::c_compile(root, true)
                .map_err(|e| eprintln!("{e}"))
                .is_ok(),
        ),
        ["verify"] => finish(xtask::verify::verify(root, 60)),
        ["verify", "--fuzz-seconds", n] => match n.parse() {
            Ok(n) => finish(xtask::verify::verify(root, n)),
            Err(_) => usage(),
        },
        _ => usage(),
    }
}

fn finish(passed: bool) -> ExitCode {
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}
