// SPDX-License-Identifier: GPL-3.0-or-later
//! Memory checks and negative controls, with isolated artifacts and pinned tools.
use crate::{Check, require};
use std::{path::Path, process::Command};

const NIGHTLY: &str = "+nightly-2026-10-30";
const TARGET: &str = "x86_64-unknown-linux-gnu";
pub const SANITIZER_INSTALL: &str = "install a sanitizer-capable C compiler and its system runtimes: libasan and libubsan on Fedora/RHEL, or your distribution's equivalent packages";

fn cargo(root: &Path, dir: &str) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root)
        .arg(NIGHTLY)
        .env("CARGO_TARGET_DIR", root.join("target").join(dir));
    cmd
}

fn success(cmd: &mut Command) -> Check {
    let status = cmd.status().map_err(|e| format!("{cmd:?}: {e}"))?;
    require(status.success(), format!("{cmd:?}: {status}"))
}

// A failed build, missing component, assertion, or unrelated crash cannot count
// as a successful negative control. Require the tool's specific memory diagnosis.
fn caught(cmd: &mut Command, log: &Path, diagnosis: &[&str]) -> Check {
    let output = cmd.output().map_err(|e| format!("{cmd:?}: {e}"))?;
    let mut bytes = output.stdout;
    bytes.extend_from_slice(&output.stderr);
    std::fs::write(log, &bytes).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    require(
        !output.status.success() && diagnosis.iter().all(|part| text.contains(part)),
        format!(
            "expected memory diagnosis {diagnosis:?}; see {}\n{text}",
            log.display()
        ),
    )?;
    println!(
        "planted heap overread caught; diagnostic: {}",
        log.display()
    );
    Ok(())
}

pub fn miri(root: &Path, planted: bool) -> Check {
    std::fs::create_dir_all(root.join("target/verify")).map_err(|e| e.to_string())?;
    // Setup failure must not be mistaken for a caught planted defect.
    success(cargo(root, "ffi-miri").args(["miri", "setup"]))?;
    let mut cmd = cargo(
        root,
        if planted {
            "ffi-miri-planted"
        } else {
            "ffi-miri"
        },
    );
    cmd.args(["miri", "test", "-p", "qso-graph-interop-ffi", "--locked"])
        // Tiny boundary cases need no filesystem access. Keep isolation and all
        // memory/aliasing checks, including strict provenance, enabled.
        .env("MIRIFLAGS", "-Zmiri-strict-provenance");
    if planted {
        cmd.env("RUSTFLAGS", "--cfg qgi_planted_read").args([
            "tests::planted_memory_read",
            "--",
            "--exact",
        ]);
        caught(
            &mut cmd,
            &root.join("target/verify/ffi-miri-planted.txt"),
            &[
                "Undefined Behavior",
                "read",
                "beyond the end of the allocation",
            ],
        )
    } else {
        success(&mut cmd)
    }
}

pub fn sanitize(root: &Path, planted: bool) -> Check {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        println!(
            "SKIP: FFI ASan/UBSan requires Linux x86_64 GNU and a sanitizer-capable C compiler"
        );
        return Ok(());
    }
    std::fs::create_dir_all(root.join("target/verify")).map_err(|e| e.to_string())?;
    let dir = if planted {
        "ffi-asan-planted"
    } else {
        "ffi-asan"
    };
    let mut build = cargo(root, dir);
    build
        .args([
            "rustc",
            "-p",
            "qso-graph-interop-ffi",
            "--lib",
            "--locked",
            "--target",
            TARGET,
            "--crate-type",
            "staticlib",
        ])
        .env(
            "RUSTFLAGS",
            if planted {
                "-Zsanitizer=address -Cforce-frame-pointers=yes --cfg qgi_planted_read"
            } else {
                "-Zsanitizer=address -Cforce-frame-pointers=yes"
            },
        );
    success(&mut build)?;
    let binary = root.join("target").join(dir).join("ffi-smoke");
    let library = root
        .join("target")
        .join(dir)
        .join(TARGET)
        .join("debug/libqso_graph_interop_ffi.a");
    let mut cc = Command::new("cc");
    cc.current_dir(root)
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-g",
            "-fno-omit-frame-pointer",
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-no-pie",
            "-Icrates/qso-graph-interop-ffi/include",
            "crates/qso-graph-interop-ffi/tests/smoke.c",
        ])
        .arg(library)
        .args(["-ldl", "-lpthread", "-lm", "-o"])
        .arg(&binary);
    success(&mut cc).map_err(|e| format!("{e}\n{SANITIZER_INSTALL}"))?;
    let mut run = Command::new(binary);
    run.current_dir(root)
        .env("ASAN_OPTIONS", "detect_leaks=1:halt_on_error=1")
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1");
    let result = if planted {
        caught(
            &mut run,
            &root.join("target/verify/ffi-asan-planted.txt"),
            &[
                "ERROR: AddressSanitizer: heap-buffer-overflow",
                "READ of size 1",
            ],
        )
    } else {
        success(&mut run)
    };
    result.map_err(|e| format!("{e}\n{SANITIZER_INSTALL}"))
}
