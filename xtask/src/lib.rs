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
                } else {
                    if name == "std" {
                        // No root/module aliases, glob imports, grouped paths, or alternate OS APIs.
                        // Module aliases for io/time are harmless; networking is restricted to the
                        // SocketAddr value type. The audited adapter is checked separately.
                        let mut segments = Vec::new();
                        let mut cursor = i + 1;
                        let colon = |index| matches!(trees.get(index), Some(TokenTree::Punct(p)) if p.as_char() == ':');
                        while colon(cursor) && colon(cursor + 1) {
                            let Some(TokenTree::Ident(segment)) = trees.get(cursor + 2) else {
                                break;
                            };
                            segments.push(segment.to_string().trim_start_matches("r#").to_owned());
                            cursor += 3;
                        }
                        let path = segments.join("::");
                        let allowed = [
                            "io",
                            "time",
                            "error",
                            "str",
                            "net::SocketAddr",
                            "process::ExitCode",
                        ];
                        require(
                            allowed.iter().any(|p| {
                                path.strip_prefix(p)
                                    .is_some_and(|tail| tail.is_empty() || tail.starts_with("::"))
                            }),
                            format!("unapproved CLI standard-library path: std::{path}"),
                        )?;
                    }
                    require(
                        name != "UdpSocket",
                        "socket APIs belong only in the audited numeric UDP adapter",
                    )?;
                    if name == "process" {
                        let prefix = trees[..i]
                            .iter()
                            .rev()
                            .take(3)
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        let suffix = trees[i + 1..]
                            .iter()
                            .take(3)
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        require(
                            prefix == [":", ":", "std"] && suffix == [":", ":", "ExitCode"],
                            "CLI process access must be std::process::ExitCode",
                        )?;
                    }
                }
            }
            TokenTree::Group(g) => network_tokens(g.stream(), library)?,
            _ => {}
        }
    }
    Ok(())
}

// This exact token template is the audited exception to the CLI socket ban.
// Changes to either the interface or implementation require updating this policy oracle.
const AUDITED_NUMERIC_UDP: &str = r#"// SPDX-License-Identifier: GPL-3.0-or-later
//! Audited socket boundary: address-taking operations accept only SocketAddr.
use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};

pub struct NumericUdp(UdpSocket);

impl NumericUdp {
    pub fn bind(address: SocketAddr) -> io::Result<Self> {
        UdpSocket::bind(address).map(Self)
    }

    pub fn sender(peer: SocketAddr) -> io::Result<Self> {
        let local = if peer.is_ipv4() {
            SocketAddr::from(([0, 0, 0, 0], 0))
        } else {
            SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))
        };
        let socket = Self::bind(local)?;
        socket.0.connect(peer)?;
        Ok(socket)
    }

    pub fn recv_from(&self, bytes: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(bytes)
    }

    pub fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        self.0.send(bytes)
    }
}
"#;

fn numeric_udp_tokens(tokens: TokenStream) -> Check {
    require(
        tokens.to_string()
            == AUDITED_NUMERIC_UDP
                .parse::<TokenStream>()
                .expect("audited template")
                .to_string(),
        "numeric UDP adapter differs from the audited template",
    )
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
                    if key == "serde_json" {
                        require(
                            value
                                .get("features")
                                .and_then(|v| v.as_array())
                                .is_some_and(|features| {
                                    features
                                        .iter()
                                        .any(|v| v.as_str() == Some("float_roundtrip"))
                                }),
                            "serde_json must explicitly enable float_roundtrip",
                        )?;
                    }
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

// A new dependency can expose process/network APIs under arbitrary names. Reject additions
// and package aliases until their public API has been reviewed with the socket boundary.
fn shipped_dependency_names(table: &toml::Table, library: bool) -> Check {
    fn scan(table: &toml::Table, library: bool) -> Check {
        for (key, value) in table {
            if matches!(
                key.as_str(),
                "dependencies" | "dev-dependencies" | "build-dependencies"
            ) {
                let dependencies = value
                    .as_table()
                    .ok_or("dependency section must be a table")?;
                for (name, specification) in dependencies {
                    let allowed: &[&str] = match (library, key.as_str()) {
                        (true, "dependencies") => &["serde"],
                        (true, "dev-dependencies") => &["serde_json"],
                        (false, "dependencies") => &["clap", "qso-graph-udp", "serde_json"],
                        _ => &[],
                    };
                    require(
                        allowed.contains(&name.as_str()),
                        format!("unreviewed shipped dependency API: {name}"),
                    )?;
                    require(
                        specification
                            .get("package")
                            .is_none_or(|package| package.as_str() == Some(name.as_str())),
                        format!("shipped dependency package alias: {name}"),
                    )?;
                }
            } else if let Some(nested) = value.as_table() {
                scan(nested, library)?;
            }
        }
        Ok(())
    }
    scan(table, library)
}

// Test roots may use processes and sockets, but cannot also be imported as modules.
fn reject_test_root_modules(tokens: TokenStream, names: &BTreeSet<String>) -> Check {
    let trees: Vec<_> = tokens.into_iter().collect();
    for (i, token) in trees.iter().enumerate() {
        match token {
            TokenTree::Ident(id) if id == "mod" => {
                if let Some(TokenTree::Ident(name)) = trees.get(i + 1) {
                    require(
                        !names.contains(name.to_string().trim_start_matches("r#")),
                        format!("test target root cannot be imported as module: {name}"),
                    )?;
                }
            }
            TokenTree::Group(group) => reject_test_root_modules(group.stream(), names)?,
            _ => {}
        }
    }
    Ok(())
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
        let package_manifest = toml_file(path)?;
        manifest_policy(&package_manifest, false)?;
        if pkg["name"] == "qso-graph-udp" || pkg["name"] == "qso-graph-udp-cli" {
            shipped_dependency_names(&package_manifest, pkg["name"] == "qso-graph-udp")?;
        }
        no_native(dir)?;
        let targets = array(pkg, "targets")?;
        let mut test_roots = BTreeSet::new();
        let mut other_roots = BTreeSet::new();
        for target in targets {
            let source = PathBuf::from(target["src_path"].as_str().ok_or("no target source")?);
            let source = source
                .canonicalize()
                .map_err(|e| format!("{}: {e}", source.display()))?;
            if array(target, "kind")?.iter().all(|kind| kind == "test") {
                test_roots.insert(source);
            } else {
                other_roots.insert(source);
            }
        }
        test_roots.retain(|source| !other_roots.contains(source));
        let test_names: BTreeSet<String> = test_roots
            .iter()
            .map(|source| {
                let name = if source.file_name().is_some_and(|name| name == "mod.rs") {
                    source.parent().and_then(Path::file_name)
                } else {
                    source.file_stem()
                };
                name.and_then(|name| name.to_str())
                    .map(str::to_owned)
                    .ok_or("invalid test root name")
            })
            .collect::<Result<_, _>>()?;
        for target in targets {
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
            let canonical_source = source
                .canonicalize()
                .map_err(|e| format!("{}: {e}", source.display()))?;
            let canonical_dir = dir
                .canonicalize()
                .map_err(|e| format!("{}: {e}", dir.display()))?;
            require(
                source.extension().is_some_and(|e| e == "rs")
                    && canonical_source.starts_with(&canonical_dir),
                "crate root must be a .rs file in its own crate",
            )?;
            require(
                forbids_unsafe(&read(source)?),
                format!("{} must say #![forbid(unsafe_code)]", source.display()),
            )?;
        }
        for file in files(dir)? {
            if file.extension().is_some_and(|e| e == "rs") {
                let tokens = token_file(&file)?;
                source_tokens(tokens.clone()).map_err(|e| format!("{}: {e}", file.display()))?;
                if pkg["name"] == "qso-graph-udp-cli" {
                    reject_test_root_modules(tokens, &test_names)
                        .map_err(|e| format!("{}: {e}", file.display()))?;
                }
                if pkg["name"] == "qso-graph-udp"
                    || pkg["name"] == "qso-graph-udp-cli"
                        && !test_roots.contains(
                            &file
                                .canonicalize()
                                .map_err(|e| format!("{}: {e}", file.display()))?,
                        )
                {
                    if pkg["name"] == "qso-graph-udp-cli" && file == dir.join("src/numeric_udp.rs")
                    {
                        numeric_udp_tokens(token_file(&file)?)?;
                    } else {
                        network_tokens(token_file(&file)?, pkg["name"] == "qso-graph-udp")?;
                    }
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
    fn planted_targets_outside_src_are_rejected() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/policy-target-planted");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = base.join(format!("{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = r#"
[workspace]
[workspace.lints.rust]
unsafe_code = "forbid"
[package]
name = "qso-graph-udp-cli"
version = "0.1.0"
edition = "2024"
[lints]
workspace = true
"#;
        let probe = r#"// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use std::process as child;
use std::net::UdpSocket;
pub fn policy_probe() { let _ = child::Command::new("true").status(); }
pub fn dns_probe() { let _ = UdpSocket::bind("localhost:0"); }
fn main() {}
"#;
        std::fs::write(dir.join("policy_probe.rs"), probe).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("{manifest}\n[[bin]]\nname = \"policy-probe\"\npath = \"policy_probe.rs\"\n"),
        )
        .unwrap();
        let get_metadata = || {
            let mut command =
                Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
            command
                .args([
                    "metadata",
                    "--no-deps",
                    "--format-version",
                    "1",
                    "--manifest-path",
                ])
                .arg(dir.join("Cargo.toml"));
            serde_json::from_str::<Value>(&output(&mut command).unwrap()).unwrap()
        };
        let result = workspace_policy(&dir, &get_metadata());
        assert!(
            result
                .unwrap_err()
                .contains("unapproved CLI standard-library path")
        );
        std::fs::write(
            dir.join("policy_probe.rs"),
            "#![forbid(unsafe_code)]\nfn main() {}\n",
        )
        .unwrap();
        std::fs::write(dir.join("build.rs"), probe).unwrap();
        let result = workspace_policy(&dir, &get_metadata());
        assert!(result.unwrap_err().contains("build scripts"));
        std::fs::remove_file(dir.join("build.rs")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), format!("{manifest}\n[[bin]]\nname = \"policy-probe\"\npath = \"policy_probe.rs\"\n[[test]]\nname = \"integration\"\npath = \"integration.rs\"\n")).unwrap();
        std::fs::write(dir.join("integration.rs"), probe).unwrap();
        assert!(workspace_policy(&dir, &get_metadata()).is_ok());
        std::fs::write(
            dir.join("policy_probe.rs"),
            "#![forbid(unsafe_code)]\nmod integration;\nfn main() {}\n",
        )
        .unwrap();
        assert!(
            workspace_policy(&dir, &get_metadata())
                .unwrap_err()
                .contains("test target root cannot be imported")
        );
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
        assert!(network_tokens("use std::net::SocketAddr;".parse().unwrap(), false).is_ok());
    }
    #[test]
    fn review_process_alias_and_implicit_dns_are_rejected() {
        for source in [
            "use std::process as child; pub fn policy_probe() { let _ = child::Command::new(\"true\").status(); }",
            "pub fn dns_probe() { let _ = UdpSocket::bind(\"localhost:0\"); }",
            "use std as s; use s::process as child;",
            "use std::{process as child};",
            "use std::process::*;",
            "use std::os::unix::process::CommandExt;",
            "use std::net as n; n::UdpSocket::bind(\"localhost:0\");",
            "use std::net::{UdpSocket as S}; S::bind(\"localhost:0\");",
            "use std::*;",
            "extern crate std as system;",
            "use std::net::TcpSocket;",
        ] {
            assert!(
                network_tokens(source.parse().unwrap(), false).is_err(),
                "{source}"
            );
        }
        assert!(network_tokens("use std::process::ExitCode;".parse().unwrap(), false).is_ok());
    }

    #[test]
    fn review_implicit_dns_is_rejected() {
        assert!(
            network_tokens(
                "pub fn dns_probe() { let _ = UdpSocket::bind(\"localhost:0\"); }"
                    .parse()
                    .unwrap(),
                false
            )
            .is_err()
        );
    }

    #[test]
    fn numeric_udp_adapter_changes_are_rejected() {
        assert!(numeric_udp_tokens(AUDITED_NUMERIC_UDP.parse().unwrap()).is_ok());
        for mutant in [
            AUDITED_NUMERIC_UDP.replace(
                "UdpSocket::bind(address)",
                "UdpSocket::bind(\"localhost:0\")",
            ),
            AUDITED_NUMERIC_UDP.replace(
                "socket.0.connect(peer)",
                "socket.0.connect(\"example.com:2237\")",
            ),
            AUDITED_NUMERIC_UDP.replace(
                "SocketAddr) -> io::Result<Self>",
                "impl std::net::ToSocketAddrs) -> io::Result<Self>",
            ),
            format!("{AUDITED_NUMERIC_UDP} pub fn extra() {{}}"),
        ] {
            assert!(numeric_udp_tokens(mutant.parse().unwrap()).is_err());
        }
    }

    #[test]
    fn dependency_policy_allows_rust_unsafe_and_macros_but_own_code_does_not() {
        let dependency = "unsafe fn internal() {} macro_rules! helper { () => {} }";
        assert!(native_tokens(dependency.parse().unwrap()).is_ok());
        assert!(source_tokens(dependency.parse().unwrap()).is_err());
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert!(
            read(&root.join("AGENTS.md"))
                .unwrap()
                .contains("Dependencies may use unsafe Rust and define macros")
        );
        assert!(
            read(&root.join("SECURITY.md"))
                .unwrap()
                .contains("Dependencies may use unsafe Rust and define macros")
        );
        assert!(
            !read(&root.join("README.md")).unwrap().contains(
                "refuses macro definitions and native code in the shipped dependency graph"
            )
        );
    }

    #[test]
    fn unreviewed_dependency_apis_aliases_and_target_paths_are_rejected() {
        for source in [
            "[dependencies]\nduct='=0.13.0'",
            "[dependencies]\nclap={package='duct',version='=0.13.0'}",
            "[target.'cfg(unix)'.dependencies]\nresolver='=1.0.0'",
            "[dev-dependencies]\nprocess_wrapper='=1.0.0'",
            "[build-dependencies]\nprocess_wrapper='=1.0.0'",
        ] {
            assert!(shipped_dependency_names(&toml::from_str(source).unwrap(), false).is_err());
        }
        assert!(
            shipped_dependency_names(
                &toml::from_str(
                    "[dependencies]\nserde='=1.0.229'\n[dev-dependencies]\nserde_json='=1.0.145'"
                )
                .unwrap(),
                true
            )
            .is_ok()
        );
        assert!(
            shipped_dependency_names(
                &toml::from_str(
                    "[dependencies]\nclap='=4.6.7'\nserde_json='=1.0.145'\nqso-graph-udp='=0.1.0'"
                )
                .unwrap(),
                false
            )
            .is_ok()
        );
        assert!(
            shipped_dependency_names(
                &toml::from_str("[dependencies]\nwrapper='=1.0.0'").unwrap(),
                true
            )
            .is_err()
        );
    }

    #[test]
    fn serde_json_requires_float_roundtrip_in_every_manifest() {
        for dependency in [
            "serde_json='=1.0.145'",
            "serde_json={version='=1.0.145',features=[]}",
        ] {
            let source = format!("[lints]\nworkspace=true\n[dependencies]\n{dependency}");
            assert!(manifest_policy(&toml::from_str(&source).unwrap(), false).is_err());
        }
        let source = "[lints]\nworkspace=true\n[dependencies]\nserde_json={version='=1.0.145',features=['float_roundtrip']}";
        assert!(manifest_policy(&toml::from_str(source).unwrap(), false).is_ok());
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
