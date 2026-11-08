// SPDX-License-Identifier: GPL-3.0-or-later
//! Policy adapted from QSO Graph's token-based Rust checks. Fail closed on unreadable input.
#![forbid(unsafe_code)]

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use serde_json::Value;
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    process::Command,
};

type Check = Result<(), String>;

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}
fn toml_file(path: &Path) -> Result<toml::Table, String> {
    toml::from_str(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}
fn require(condition: bool, message: impl Into<String>) -> Check {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn output(command: &mut Command) -> Result<String, String> {
    let result = command.output().map_err(|e| e.to_string())?;
    require(
        result.status.success(),
        String::from_utf8_lossy(&result.stderr).into_owned(),
    )?;
    String::from_utf8(result.stdout).map_err(|e| e.to_string())
}
fn metadata(root: &Path) -> Result<Value, String> {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args([
            "metadata",
            "--locked",
            "--all-features",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"));
    serde_json::from_str(&output(&mut command)?).map_err(|e| e.to_string())
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    value[key]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("metadata missing {key}"))
}

/// Parse the actual inner attribute, so a comment or string cannot stand in for it.
pub fn forbids_unsafe(source: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    if file
        .attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
    {
        return false;
    }
    file.attrs.iter().any(|a| {
        matches!(a.style, syn::AttrStyle::Inner(_))
            && a.path().is_ident("forbid")
            && a.parse_args::<syn::Ident>()
                .is_ok_and(|i| i == "unsafe_code")
    })
}

/// Scan all tokens, including disabled cfg branches; comments and strings are not code.
/// Macro definitions, includes and path redirection cannot hide source from the scan.
pub fn source_tokens(tokens: TokenStream) -> Check {
    fn scan(tokens: TokenStream, in_attribute: bool) -> Check {
        let trees: Vec<_> = tokens.into_iter().collect();
        let punct =
            |i: usize, c: char| matches!(trees.get(i),Some(TokenTree::Punct(p)) if p.as_char()==c);
        for (i, token) in trees.iter().enumerate() {
            match token {
                TokenTree::Ident(id) => {
                    let name = id.to_string();
                    let name = name.trim_start_matches("r#");
                    require(
                        !matches!(
                            name,
                            "unsafe"
                                | "macro_rules"
                                | "macro"
                                | "include"
                                | "asm"
                                | "global_asm"
                                | "naked_asm"
                        ),
                        format!("forbidden source token {name}"),
                    )?;
                    if in_attribute {
                        require(
                            !matches!(name, "path" | "feature" | "allow" | "expect" | "warn"),
                            format!("attribute can bypass policy: {name}"),
                        )?;
                    }
                }
                TokenTree::Group(g) => {
                    let attribute = g.delimiter() == Delimiter::Bracket
                        && i > 0
                        && (punct(i - 1, '#') || i > 1 && punct(i - 1, '!') && punct(i - 2, '#'));
                    let first = g
                        .stream()
                        .into_iter()
                        .next()
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    // Rust's prose attributes and clap help text contain no Rust code.
                    if attribute && !matches!(first.as_str(), "doc" | "test" | "command" | "arg") {
                        for item in g.stream() {
                            if let TokenTree::Literal(l) = item
                                && let Ok(lit) = syn::parse_str::<syn::LitStr>(&l.to_string())
                                && let Ok(inner) = lit.value().parse::<TokenStream>()
                            {
                                scan(inner, false)?;
                            }
                        }
                    }
                    scan(g.stream(), in_attribute || attribute)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    scan(tokens, false)
}

pub fn native_tokens(tokens: TokenStream) -> Check {
    let trees: Vec<_> = tokens.into_iter().collect();
    for (i, t) in trees.iter().enumerate() {
        match t {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                let name = name.trim_start_matches("r#");
                require(
                    !matches!(name, "asm" | "global_asm" | "naked_asm"),
                    "native assembly",
                )?;
                if name == "extern" {
                    let next = trees.get(i + 1);
                    let after = trees.get(i + 2);
                    let brace = |t: Option<&TokenTree>| matches!(t, Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace);
                    let rust_abi =
                        matches!(next, Some(TokenTree::Literal(l)) if l.to_string() == "\"Rust\"");
                    require(
                        rust_abi
                            || !(brace(next)
                                || matches!(next, Some(TokenTree::Literal(_))) && brace(after)),
                        "native foreign block",
                    )?;
                }
            }
            TokenTree::Group(g) => {
                let first = g
                    .stream()
                    .into_iter()
                    .next()
                    .map(|t| t.to_string())
                    .unwrap_or_default();
                require(
                    g.delimiter() != Delimiter::Bracket
                        || !matches!(first.as_str(), "link" | "link_name" | "link_section"),
                    "native link attribute",
                )?;
                native_tokens(g.stream())?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// The codec library has no sockets or processes. The CLI may use only UDP with
/// numeric addresses; process creation and TCP/DNS APIs are forbidden there too.
pub fn network_tokens(tokens: TokenStream, library: bool) -> Check {
    let trees: Vec<_> = tokens.into_iter().collect();
    for (i, token) in trees.iter().enumerate() {
        match token {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                let name = name.trim_start_matches("r#");
                require(
                    !matches!(
                        name,
                        "TcpStream"
                            | "TcpListener"
                            | "ToSocketAddrs"
                            | "lookup_host"
                            | "UnixStream"
                            | "UnixDatagram"
                    ),
                    format!("network API {name}"),
                )?;
                if library {
                    require(
                        !matches!(name, "net" | "UdpSocket" | "process"),
                        format!("library API {name}"),
                    )?;
                } else if name == "process" {
                    let remaining = trees
                        .iter()
                        .skip(i + 1)
                        .take(4)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>();
                    require(
                        !remaining.iter().any(|s| s == "Command"),
                        "process creation",
                    )?;
                }
            }
            TokenTree::Group(g) => network_tokens(g.stream(), library)?,
            _ => {}
        }
    }
    Ok(())
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut stack = vec![dir.to_owned()];
    let mut found = Vec::new();
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            require(
                !kind.is_symlink(),
                format!("symbolic link: {}", entry.path().display()),
            )?;
            if kind.is_dir() {
                stack.push(entry.path());
            } else {
                found.push(entry.path());
            }
        }
    }
    Ok(found)
}
fn token_file(path: &Path) -> Result<TokenStream, String> {
    read(path)?
        .parse()
        .map_err(|e| format!("{} does not tokenize: {e}", path.display()))
}
fn no_native(dir: &Path) -> Check {
    for path in files(dir)? {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        require(
            !matches!(
                ext.as_str(),
                "c" | "cc"
                    | "cpp"
                    | "cxx"
                    | "h"
                    | "hpp"
                    | "s"
                    | "asm"
                    | "o"
                    | "obj"
                    | "a"
                    | "lib"
                    | "so"
                    | "dll"
                    | "dylib"
            ),
            format!("native code: {}", path.display()),
        )?;
        if ext == "rs" {
            native_tokens(token_file(&path)?).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

pub fn manifest_policy(t: &toml::Table, fuzz: bool) -> Check {
    require(
        t.get("lints").and_then(|v| {
            if fuzz {
                v.get("rust")
                    .and_then(|v| v.get("unsafe_code"))
                    .and_then(|v| v.as_str())
                    .map(|s| s == "forbid")
            } else {
                v.get("workspace").and_then(|v| v.as_bool())
            }
        }) == Some(true),
        "crate must inherit lints (fuzz must forbid unsafe)",
    )?;
    fn dependencies(v: &toml::Value, in_deps: bool) -> Check {
        if let Some(table) = v.as_table() {
            for (key, value) in table {
                if in_deps {
                    let version = value
                        .as_str()
                        .or_else(|| value.get("version").and_then(|v| v.as_str()));
                    require(
                        version.is_some_and(exact_version),
                        format!("dependency {key} must use an exact = version"),
                    )?;
                    require(
                        value.get("git").is_none() && value.get("registry").is_none(),
                        format!("dependency {key} must come from crates.io"),
                    )?;
                } else {
                    dependencies(
                        value,
                        matches!(
                            key.as_str(),
                            "dependencies" | "dev-dependencies" | "build-dependencies"
                        ),
                    )?;
                }
            }
        }
        Ok(())
    }
    dependencies(&toml::Value::Table(t.clone()), false)
}
fn exact_version(v: &str) -> bool {
    v.strip_prefix('=')
        .is_some_and(|s| !s.is_empty() && !s.contains([',', '*', '^', '~', ' ', '>', '<']))
}

fn workspace_policy(root: &Path, meta: &Value) -> Check {
    let manifest = toml_file(&root.join("Cargo.toml"))?;
    require(
        manifest
            .get("workspace")
            .and_then(|v| v.get("lints"))
            .and_then(|v| v.get("rust"))
            .and_then(|v| v.get("unsafe_code"))
            .and_then(|v| v.as_str())
            == Some("forbid"),
        "workspace must forbid unsafe_code",
    )?;
    let members = array(meta, "workspace_members")?;
    for pkg in array(meta, "packages")? {
        if !members.contains(&pkg["id"]) {
            continue;
        }
        let path = Path::new(pkg["manifest_path"].as_str().ok_or("no manifest")?);
        let dir = path.parent().ok_or("no parent")?;
        manifest_policy(&toml_file(path)?, false)?;
        no_native(dir)?;
        for target in array(pkg, "targets")? {
            require(
                target["edition"] == "2024",
                "every target must be edition 2024",
            )?;
            require(
                !array(target, "kind")?
                    .iter()
                    .any(|k| k == "proc-macro" || k == "custom-build"),
                "workspace cannot define proc macros or build scripts",
            )?;
            let source = Path::new(target["src_path"].as_str().ok_or("no target source")?);
            require(
                source.extension().is_some_and(|e| e == "rs") && source.starts_with(dir),
                "crate root must be a .rs file in its own crate",
            )?;
            require(
                forbids_unsafe(&read(source)?),
                format!("{} must say #![forbid(unsafe_code)]", source.display()),
            )?;
        }
        for file in files(dir)? {
            if file.extension().is_some_and(|e| e == "rs") {
                source_tokens(token_file(&file)?)
                    .map_err(|e| format!("{}: {e}", file.display()))?;
                if pkg["name"] == "qso-graph-udp"
                    || pkg["name"] == "qso-graph-udp-cli" && file.starts_with(dir.join("src"))
                {
                    network_tokens(token_file(&file)?, pkg["name"] == "qso-graph-udp")?;
                }
            }
        }
    }
    Ok(())
}

fn dependency_policy(meta: &Value) -> Check {
    let packages = array(meta, "packages")?;
    let nodes = array(&meta["resolve"], "nodes")?;
    let mut queue: VecDeque<&Value> = packages
        .iter()
        .filter(|p| p["name"] == "qso-graph-udp" || p["name"] == "qso-graph-udp-cli")
        .map(|p| &p["id"])
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id.to_string()) {
            continue;
        }
        let pkg = packages
            .iter()
            .find(|p| &p["id"] == id)
            .ok_or("missing package")?;
        if !pkg["source"].is_null() {
            require(
                pkg["source"] == "registry+https://github.com/rust-lang/crates.io-index",
                "only crates.io sources allowed",
            )?;
            let manifest = Path::new(pkg["manifest_path"].as_str().ok_or("missing manifest")?);
            no_native(manifest.parent().ok_or("missing crate directory")?)?;
            // Build scripts and procedural macros are subject to cargo-vet, including exact
            // versions that remain explicitly not audited in supply-chain/config.toml.
        } else {
            require(
                pkg["name"] == "qso-graph-udp" || pkg["name"] == "qso-graph-udp-cli",
                "unexpected path dependency in shipped graph",
            )?;
        }
        let node = nodes
            .iter()
            .find(|n| &n["id"] == id)
            .ok_or("missing dependency node")?;
        queue.extend(array(node, "dependencies")?);
    }
    Ok(())
}

fn compiler_policy(root: &Path) -> Check {
    let t = toml_file(&root.join("rust-toolchain.toml"))?;
    require(
        t["toolchain"]["channel"].as_str() == Some("1.99.1"),
        "compiler must be pinned to 1.99.1",
    )?;
    let version = output(Command::new("rustc").arg("--version"))?;
    require(
        version.starts_with("rustc 1.99.1 "),
        format!("wrong compiler: {version}"),
    )?;
    for (key, value) in std::env::vars() {
        if key.contains("RUSTFLAGS") {
            require(
                value.is_empty() || value == "-D warnings" || value == "-D\u{1f}warnings",
                format!("unreviewed flags: {key}"),
            )?;
        }
        if key.contains("RUSTC_WRAPPER")
            || key.contains("RUSTC_WORKSPACE_WRAPPER")
            || key == "RUSTC_BOOTSTRAP"
        {
            require(value.is_empty(), format!("compiler override {key}"))?;
        }
    }
    let config = toml_file(&root.join(".cargo/config.toml"))?;
    require(
        config.keys().all(|k| k == "alias"),
        "cargo config must contain only aliases",
    )?;
    Ok(())
}
fn fuzz_policy(root: &Path) -> Check {
    let t = toml_file(&root.join("fuzz/rust-toolchain.toml"))?;
    require(
        t["toolchain"]["channel"].as_str() == Some("nightly-2026-10-30"),
        "fuzz nightly must be pinned",
    )?;
    let manifest = toml_file(&root.join("fuzz/Cargo.toml"))?;
    manifest_policy(&manifest, true)?;
    require(
        manifest["workspace"].is_table(),
        "fuzz must have its own workspace",
    )?;
    require(
        root.join("fuzz/Cargo.lock").is_file(),
        "fuzz lockfile is required",
    )?;
    for bin in manifest["bin"].as_array().ok_or("fuzz targets missing")? {
        let path = root
            .join("fuzz")
            .join(bin["path"].as_str().ok_or("missing target path")?);
        require(
            forbids_unsafe(&read(&path)?),
            "fuzz root must forbid unsafe",
        )?;
        source_tokens(token_file(&path)?)?;
    }
    Ok(())
}

pub fn policy(root: &Path) -> Check {
    let meta = metadata(root)?;
    compiler_policy(root)?;
    println!("policy: pinned compiler and flags: pass");
    workspace_policy(root, &meta)?;
    println!("policy: roots, lints, tokens, dependency pins, codec without networking: pass");
    dependency_policy(&meta)?;
    println!("policy: crates.io only, no native code in shipped dependency graph: pass");
    fuzz_policy(root)?;
    println!("policy: isolated, pinned fuzz target: pass");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planted_unsafe_and_macro_cases() {
        for source in [
            "unsafe {}",
            "#[cfg(any())] unsafe fn hidden() {}",
            "macro_rules! m { () => {} }",
            "r#macro_rules! m {}",
            "include!(\"hidden\")",
            "#[path=\"hidden\"] mod m;",
            "#[allow(unsafe_code)] fn f() {}",
            "#[cfg_attr(any(), allow(unsafe_code))] fn f() {}",
            "asm!(\"\")",
        ] {
            assert!(source_tokens(source.parse().unwrap()).is_err(), "{source}");
        }
        assert!(
            source_tokens(
                "// unsafe\nlet message = \"unsafe macro_rules!\";"
                    .parse()
                    .unwrap()
            )
            .is_ok()
        );
    }
    #[test]
    fn planted_fake_crate_root_attributes() {
        assert!(forbids_unsafe("#![forbid(unsafe_code)]\npub fn f() {}"));
        for s in [
            "// #![forbid(unsafe_code)]",
            "#![forbid(unsafe_code)] #![cfg(any())]",
            "const S: &str = \"#![forbid(unsafe_code)]\";",
            "#[forbid(unsafe_code)] fn f() {}",
        ] {
            assert!(!forbids_unsafe(s));
        }
    }
    #[test]
    fn planted_native_links_and_sources() {
        for source in [
            "unsafe extern \"C\" { fn f(); }",
            "#[link(name=\"ssl\")] extern {}",
            "core::arch::asm!(\"\")",
        ] {
            assert!(native_tokens(source.parse().unwrap()).is_err());
        }
        assert!(native_tokens("extern \"Rust\" { fn f(); }".parse().unwrap()).is_ok());
    }
    #[test]
    fn planted_native_file_is_refused() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/policy-planted");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = base.join(format!("{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("allowed.rs"), "pub fn f() {}").unwrap();
        assert!(no_native(&dir).is_ok());
        std::fs::write(dir.join("hidden.c"), "int f(void) { return 0; }").unwrap();
        assert!(no_native(&dir).is_err());
        std::fs::remove_file(dir.join("hidden.c")).unwrap();
        assert!(no_native(&dir).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn planted_network_cases() {
        for s in [
            "use std::net as n;",
            "use std::net::{UdpSocket as S};",
            "std::process::Command::new(\"curl\");",
        ] {
            assert!(network_tokens(s.parse().unwrap(), true).is_err());
        }
        for s in [
            "std::net::TcpStream::connect(\"example.com\");",
            "use std::net::ToSocketAddrs;",
            "std::process::Command::new(\"curl\");",
        ] {
            assert!(network_tokens(s.parse().unwrap(), false).is_err());
        }
        assert!(
            network_tokens(
                "use std::net::{UdpSocket, SocketAddr};".parse().unwrap(),
                false
            )
            .is_ok()
        );
    }
    #[test]
    fn planted_floating_dependency_and_lint_override() {
        let good: toml::Table =
            toml::from_str("[lints]\nworkspace=true\n[dependencies]\nserde='=1.0.229'").unwrap();
        assert!(manifest_policy(&good, false).is_ok());
        for s in [
            "[dependencies]\nserde='1'",
            "[lints]\nworkspace=true\n[dependencies]\nserde='^1'",
            "[lints]\nworkspace=true\n[dependencies]\nserde={version='=1.0.229',git='https://example.com'}",
        ] {
            assert!(manifest_policy(&toml::from_str(s).unwrap(), false).is_err());
        }
    }
}
