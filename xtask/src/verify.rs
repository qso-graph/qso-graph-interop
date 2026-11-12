// SPDX-License-Identifier: GPL-3.0-or-later
//! `cargo xtask verify`: every check this repository has, run in order, one pass/fail table.
//!
//! Every step runs even after an earlier one fails, so the table is always complete. Each
//! step's output goes to `target/verify/<step>.log`; a failed step's last lines are printed.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// One check: a program and its arguments, run in a directory relative to the workspace root.
#[derive(Debug, Clone)]
pub struct Step {
    pub name: &'static str,
    pub dir: &'static str,
    pub program: String,
    pub args: Vec<String>,
    /// How to get the program if it is missing, shown instead of a bare "not found".
    pub install: &'static str,
    /// A cargo subcommand's binary that must be on `PATH`. Cargo runs it, because tools such
    /// as cargo-vet need the environment cargo sets for them.
    pub requires: Option<&'static str>,
}

/// The result of one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(String),
}

fn step(
    name: &'static str,
    dir: &'static str,
    program: &str,
    args: &[&str],
    install: &'static str,
) -> Step {
    Step {
        name,
        dir,
        program: program.to_owned(),
        args: args.iter().map(|a| (*a).to_owned()).collect(),
        install,
        requires: None,
    }
}

fn subcommand(
    name: &'static str,
    args: &[&str],
    tool: &'static str,
    install: &'static str,
) -> Step {
    Step {
        requires: Some(tool),
        ..step(name, ".", "cargo", args, install)
    }
}

/// The checks CI runs, in the same order, so a local pass predicts a CI pass.
pub fn steps(fuzz_seconds: u64) -> Vec<Step> {
    const RUSTUP: &str = "install the pinned toolchain: rustup toolchain install";
    let fuzz_time = format!("-max_total_time={fuzz_seconds}");
    let mut list = vec![
        step("fmt", ".", "cargo", &["fmt", "--all", "--check"], RUSTUP),
        step(
            "clippy",
            ".",
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            RUSTUP,
        ),
        step(
            "test",
            ".",
            "cargo",
            &["test", "--workspace", "--all-features", "--locked"],
            RUSTUP,
        ),
        step(
            "clippy core only",
            ".",
            "cargo",
            &[
                "clippy",
                "-p",
                "qso-graph-interop",
                "--all-targets",
                "--no-default-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            RUSTUP,
        ),
        step(
            "test core only",
            ".",
            "cargo",
            &[
                "test",
                "-p",
                "qso-graph-interop",
                "--no-default-features",
                "--locked",
            ],
            RUSTUP,
        ),
        step("policy", ".", "cargo", &["xtask", "policy"], RUSTUP),
        step(
            "fixtures",
            ".",
            "python3",
            &["scripts/generate-fixtures.py", "--check"],
            "install Python 3",
        ),
        step(
            "spec vectors",
            ".",
            "uv",
            &[
                "run",
                "--no-project",
                "--with",
                "jsonschema[format-nongpl]==4.26.0",
                "python",
                "scripts/gen_vectors.py",
                "--check",
                "--producer-strict",
            ],
            "install uv: https://docs.astral.sh/uv/",
        ),
        subcommand(
            "cargo vet",
            &["vet", "--locked"],
            "cargo-vet",
            "cargo install cargo-vet --version 0.10.3 --locked",
        ),
        subcommand(
            "cargo deny",
            &["deny", "--locked", "check", "bans", "licenses", "sources"],
            "cargo-deny",
            "cargo install cargo-deny --version 0.20.2 --locked",
        ),
        subcommand(
            "cargo audit",
            &["audit", "--file", "Cargo.lock"],
            "cargo-audit",
            "cargo install cargo-audit --version 0.22.2 --locked",
        ),
    ];
    if fuzz_seconds > 0 {
        const NIGHTLY: &str = "rustup toolchain install nightly-2026-10-30 && cargo install cargo-fuzz --version 0.13.2 --locked";
        list.extend([
            step("fuzz fmt", "fuzz", "cargo", &["fmt", "--check"], NIGHTLY),
            step(
                "fuzz clippy",
                "fuzz",
                "cargo",
                &[
                    "clippy",
                    "--all-targets",
                    "--locked",
                    "--",
                    "-D",
                    "warnings",
                ],
                NIGHTLY,
            ),
            step(
                "fuzz seed",
                ".",
                "python3",
                &["fuzz/seed.py"],
                "install Python 3",
            ),
            Step {
                name: "fuzz run",
                dir: "fuzz",
                program: "cargo".to_owned(),
                args: [
                    "fuzz",
                    "run",
                    "wsjtx",
                    "--",
                    &fuzz_time,
                    "-max_len=65508",
                    "-timeout=10",
                    "-rss_limit_mb=2048",
                    "-print_final_stats=1",
                ]
                .iter()
                .map(|a| (*a).to_owned())
                .collect(),
                install: NIGHTLY,
                requires: Some("cargo-fuzz"),
            },
        ]);
    }
    list
}

/// The lockfiles a run must leave as it found them. `--locked` covers the workspace; the
/// fuzz run does not take it, so both are compared before and after.
const LOCKFILES: [&str; 2] = ["Cargo.lock", "fuzz/Cargo.lock"];

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths)
            .any(|dir| dir.join(tool).is_file() || dir.join(format!("{tool}.exe")).is_file())
    })
}

/// Variables `cargo run` sets for xtask itself. Passing them on would pin every nested cargo to
/// this toolchain and package, so the fuzz workspace's own nightly would be ignored.
fn clean_env(command: &mut Command) {
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        if matches!(
            key,
            "RUSTUP_TOOLCHAIN"
                | "CARGO"
                | "RUSTC"
                | "RUSTDOC"
                | "CARGO_PRIMARY_PACKAGE"
                | "CARGO_BIN_NAME"
                | "CARGO_CRATE_NAME"
        ) || key.starts_with("CARGO_PKG_")
            || key.starts_with("CARGO_MANIFEST_")
        {
            command.env_remove(key);
        }
    }
}

/// Run one step, writing its combined output to `log`.
pub fn run_step(root: &Path, s: &Step, log: &Path) -> Outcome {
    let file = match fs::File::create(log) {
        Ok(f) => f,
        Err(e) => return Outcome::Fail(format!("cannot write {}: {e}", log.display())),
    };
    let stderr = match file.try_clone() {
        Ok(f) => f,
        Err(e) => return Outcome::Fail(format!("cannot write {}: {e}", log.display())),
    };
    let mut command = Command::new(&s.program);
    command
        .args(&s.args)
        .current_dir(root.join(s.dir))
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(stderr);
    clean_env(&mut command);
    if let Some(tool) = s.requires
        && !on_path(tool)
    {
        return Outcome::Fail(format!("{tool} not found; {}", s.install));
    }
    match command.status() {
        Ok(status) if status.success() => Outcome::Pass,
        Ok(status) => Outcome::Fail(format!(
            "exit {}",
            status
                .code()
                .map_or_else(|| "by signal".to_owned(), |c| c.to_string())
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Outcome::Fail(format!("{} not found; {}", s.program, s.install))
        }
        Err(e) => Outcome::Fail(format!("{}: {e}", s.program)),
    }
}

fn log_name(index: usize, name: &str) -> String {
    format!("{:02}-{}.log", index + 1, name.replace(' ', "-"))
}

fn tail(path: &Path, lines: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

fn seconds(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f64())
}

/// Run every step and print the table. Returns true when all passed.
pub fn verify(root: &Path, fuzz_seconds: u64) -> bool {
    let logs: PathBuf = root.join("target/verify");
    if let Err(e) = fs::create_dir_all(&logs) {
        eprintln!("verify: cannot create {}: {e}", logs.display());
        return false;
    }
    let list = steps(fuzz_seconds);
    let read_locks = || LOCKFILES.map(|f| fs::read(root.join(f)).ok());
    let locks_before = read_locks();
    let started = Instant::now();
    let mut results = Vec::with_capacity(list.len());
    for (i, s) in list.iter().enumerate() {
        print!("[{:>2}/{}] {:<20} ", i + 1, list.len() + 1, s.name);
        let _ = std::io::stdout().flush();
        let begun = Instant::now();
        let outcome = run_step(root, s, &logs.join(log_name(i, s.name)));
        let took = begun.elapsed();
        println!(
            "{} {}",
            if outcome == Outcome::Pass {
                "pass"
            } else {
                "FAIL"
            },
            seconds(took)
        );
        results.push((s.name, outcome, took));
    }
    let lock_check = Instant::now();
    let changed: Vec<&str> = LOCKFILES
        .iter()
        .zip(locks_before.iter().zip(read_locks().iter()))
        .filter(|(_, (before, after))| before.is_none() || before != after)
        .map(|(f, _)| *f)
        .collect();
    let lock_outcome = if changed.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("changed or unreadable: {}", changed.join(", ")))
    };
    println!(
        "[{:>2}/{}] {:<20} {}",
        list.len() + 1,
        list.len() + 1,
        "lockfiles unchanged",
        if lock_outcome == Outcome::Pass {
            "pass"
        } else {
            "FAIL"
        }
    );
    results.push(("lockfiles unchanged", lock_outcome, lock_check.elapsed()));
    let failed: Vec<_> = results
        .iter()
        .enumerate()
        .filter(|(_, (_, o, _))| *o != Outcome::Pass)
        .collect();
    for (i, (name, outcome, _)) in &failed {
        let log = logs.join(log_name(*i, name));
        if let Outcome::Fail(why) = outcome {
            println!("\n--- {name} failed: {why}");
            if *i < list.len() {
                println!("--- log: {}\n{}", log.display(), tail(&log, 30));
            }
        }
    }
    println!("\n| Check | Result | Time |\n|---|---|---|");
    for (name, outcome, took) in &results {
        let result = match outcome {
            Outcome::Pass => "pass".to_owned(),
            Outcome::Fail(why) => format!("FAIL ({why})"),
        };
        println!("| {name} | {result} | {} |", seconds(*took));
    }
    let passed = results.len() - failed.len();
    println!(
        "\nverify: {passed}/{} passed in {}; logs in {}\nverify: {}",
        results.len(),
        seconds(started.elapsed()),
        logs.display(),
        if failed.is_empty() { "PASS" } else { "FAIL" }
    );
    failed.is_empty()
}

/// The command line a step runs, as CI's workflow would spell it. The fuzz time is a parameter
/// in both places, so it is compared as a placeholder.
pub fn command_line(s: &Step) -> String {
    let mut line = s.program.clone();
    for arg in &s.args {
        line.push(' ');
        line.push_str(if arg.starts_with("-max_total_time=") {
            "-max_total_time=N"
        } else {
            arg
        });
    }
    line
}

/// The check commands in CI's workflow: every `run:` line that starts a check. Toolchain setup,
/// tool installation, shell control lines and the lockfile diff (which verify does in-process)
/// are not checks.
pub fn ci_commands(workflow: &str) -> std::collections::BTreeSet<String> {
    let mut found = std::collections::BTreeSet::new();
    let mut in_block = None;
    for raw in workflow.lines() {
        let indent = raw.len() - raw.trim_start().len();
        let line = raw.trim();
        let body = line.strip_prefix("- ").unwrap_or(line);
        let command = if let Some(rest) = body.strip_prefix("run:") {
            let rest = rest.trim();
            if rest == "|" {
                in_block = Some(indent);
                continue;
            }
            in_block = None;
            rest
        } else if let Some(start) = in_block {
            if !line.is_empty() && indent <= start {
                in_block = None;
                continue;
            }
            line
        } else {
            continue;
        };
        let checks = ["cargo ", "python3 ", "uv "];
        if checks.iter().any(|p| command.starts_with(p)) && !command.starts_with("cargo install ") {
            let mut normal: Vec<String> = Vec::new();
            for word in command.split_whitespace() {
                normal.push(if word.starts_with("-max_total_time=") {
                    "-max_total_time=N".to_owned()
                } else {
                    // Shell quoting is not part of the argument.
                    word.trim_matches(['\'', '"']).to_owned()
                });
            }
            found.insert(normal.join(" "));
        }
    }
    found
}

/// Checks verify runs that CI does not, and checks CI runs that verify does not.
pub fn drift(workflow: &str, list: &[Step]) -> (Vec<String>, Vec<String>) {
    let ci = ci_commands(workflow);
    let local: std::collections::BTreeSet<String> = list.iter().map(command_line).collect();
    (
        local.difference(&ci).cloned().collect(),
        ci.difference(&local).cloned().collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workflow() -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap()
    }

    #[test]
    fn ci_and_verify_run_the_same_checks() {
        let (only_verify, only_ci) = drift(&workflow(), &steps(60));
        assert!(
            only_verify.is_empty() && only_ci.is_empty(),
            "verify only: {only_verify:?}\nCI only: {only_ci:?}"
        );
    }

    #[test]
    fn planted_drift_is_caught_both_ways() {
        let ci = workflow();
        let dropped = ci.replace("      - run: cargo xtask policy\n", "");
        assert_ne!(dropped, ci, "the planted removal must change the workflow");
        assert_eq!(drift(&dropped, &steps(60)).0, ["cargo xtask policy"]);
        let added = ci.replace(
            "      - run: cargo xtask policy\n",
            "      - run: cargo xtask policy\n      - run: cargo test --doc\n",
        );
        assert_eq!(drift(&added, &steps(60)).1, ["cargo test --doc"]);
        let block = "        run: |\n          cargo install cargo-vet --version 0.10.3 --locked --quiet\n          cargo vet --locked\n";
        assert!(ci_commands(block).contains("cargo vet --locked"));
        assert!(
            !ci_commands(block)
                .iter()
                .any(|c| c.starts_with("cargo install"))
        );
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qgi-verify-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn planted_failures_are_reported_not_passed() {
        let dir = scratch("fail");
        let bad = step("bad flag", ".", "cargo", &["--no-such-flag-planted"], "n/a");
        assert!(matches!(
            run_step(&dir, &bad, &dir.join("bad.log")),
            Outcome::Fail(_)
        ));
        let missing = step(
            "missing",
            ".",
            "qgi-planted-no-such-tool",
            &[],
            "install it",
        );
        match run_step(&dir, &missing, &dir.join("missing.log")) {
            Outcome::Fail(why) => assert!(
                why.contains("not found") && why.contains("install it"),
                "{why}"
            ),
            Outcome::Pass => panic!("a missing tool passed"),
        }
        let missing_subcommand = subcommand(
            "missing subcommand",
            &["planted"],
            "cargo-qgi-planted-no-such-tool",
            "install the subcommand",
        );
        match run_step(&dir, &missing_subcommand, &dir.join("sub.log")) {
            Outcome::Fail(why) => assert!(why.contains("install the subcommand"), "{why}"),
            Outcome::Pass => panic!("a missing cargo subcommand passed"),
        }
        let good = step("good", ".", "cargo", &["--version"], "n/a");
        assert_eq!(run_step(&dir, &good, &dir.join("good.log")), Outcome::Pass);
        assert!(
            fs::read_to_string(dir.join("good.log"))
                .unwrap()
                .starts_with("cargo ")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn steps_match_ci_and_fuzz_is_optional() {
        let names: Vec<_> = steps(60).iter().map(|s| s.name).collect();
        for required in [
            "fmt",
            "clippy",
            "test",
            "test core only",
            "policy",
            "spec vectors",
            "cargo vet",
            "cargo deny",
            "cargo audit",
            "fuzz run",
        ] {
            assert!(names.contains(&required), "{required} missing");
        }
        assert!(!steps(0).iter().any(|s| s.dir == "fuzz"));
        assert!(
            steps(60)
                .iter()
                .any(|s| s.args.iter().any(|a| a == "-max_total_time=60"))
        );
    }
}
