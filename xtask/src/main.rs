// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use std::{path::Path, process::ExitCode};

fn main() -> ExitCode {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["policy"] {
        eprintln!("usage: cargo xtask policy");
        return ExitCode::from(2);
    }
    match xtask::policy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace"),
    ) {
        Ok(()) => {
            println!("policy: all checks passed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("policy: FAIL: {e}");
            ExitCode::FAILURE
        }
    }
}
