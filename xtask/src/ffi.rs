// SPDX-License-Identifier: GPL-3.0-or-later
//! Header generation and the narrowly scoped first-party unsafe exception.
use crate::{Check, files, require};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{path::Path, process::Command};

pub fn root_policy(source: &str) -> Check {
    let file = syn::parse_file(source).map_err(|e| e.to_string())?;
    require(
        !file
            .attrs
            .iter()
            .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr")),
        "conditional FFI crate root",
    )?;
    require(
        file.attrs.iter().any(|a| {
            matches!(a.style, syn::AttrStyle::Inner(_))
                && a.path().is_ident("deny")
                && a.parse_args::<syn::Ident>()
                    .is_ok_and(|i| i == "unsafe_op_in_unsafe_fn")
        }),
        "FFI root must deny unsafe_op_in_unsafe_fn",
    )
}

/// Inspect tokens even in disabled code; require an immediately preceding SAFETY
/// line comment for each unsafe block. Strings and unrelated comments cannot pass.
pub fn unsafe_comments(source: &str) -> Check {
    // Locate real line comments, excluding literals and nested block comments.
    // Token spans identify even raw/multiline strings without guessing delimiters.
    fn literals(tokens: TokenStream, starts: &[usize], ranges: &mut Vec<(usize, usize)>) {
        for token in tokens {
            match token {
                TokenTree::Literal(literal) => {
                    let begin = literal.span().start();
                    let end = literal.span().end();
                    ranges.push((
                        starts[begin.line - 1] + begin.column,
                        starts[end.line - 1] + end.column,
                    ));
                }
                TokenTree::Group(group) => literals(group.stream(), starts, ranges),
                _ => {}
            }
        }
    }
    let tokens: TokenStream = source.parse().map_err(|e| format!("FFI tokens: {e}"))?;
    let mut starts = vec![0];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(i, _)| i + 1),
    );
    let mut ranges = Vec::new();
    literals(tokens.clone(), &starts, &mut ranges);
    ranges.sort_unstable();
    let mut comments = std::collections::BTreeSet::new();
    let bytes = source.as_bytes();
    let (mut at, mut depth, mut range) = (0, 0_usize, 0);
    while at < bytes.len() {
        while range < ranges.len() && ranges[range].1 <= at {
            range += 1;
        }
        if depth == 0 && range < ranges.len() && ranges[range].0 == at {
            at = ranges[range].1;
            continue;
        }
        if bytes[at..].starts_with(b"/*") {
            depth += 1;
            at += 2;
        } else if depth > 0 && bytes[at..].starts_with(b"*/") {
            depth -= 1;
            at += 2;
        } else if depth == 0 && bytes[at..].starts_with(b"//") {
            let end = bytes[at..]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(bytes.len(), |n| at + n);
            if source[at..end].starts_with("// SAFETY:")
                && source[at..end].trim().len() > "// SAFETY:".len()
            {
                comments.insert(starts.partition_point(|n| *n <= at));
            }
            at = end;
        } else {
            at += 1;
        }
    }
    fn scan(
        tokens: TokenStream,
        lines: &[&str],
        used: &mut std::collections::BTreeSet<usize>,
        comments: &std::collections::BTreeSet<usize>,
    ) -> Check {
        let trees: Vec<_> = tokens.into_iter().collect();
        for (i, tree) in trees.iter().enumerate() {
            if matches!(tree, TokenTree::Ident(id) if id == "unsafe")
                && matches!(trees.get(i+1), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
            {
                let line = tree.span().start().line;
                let mut previous = line.saturating_sub(1);
                let mut safety = false;
                while previous > 0 {
                    let text = lines[previous - 1].trim();
                    if !text.starts_with("//") {
                        break;
                    }
                    safety |= comments.contains(&previous);
                    previous -= 1;
                }
                require(
                    safety && used.insert(line),
                    format!("unsafe block at line {line} requires // SAFETY: justification"),
                )?;
            }
            if let TokenTree::Group(g) = tree {
                scan(g.stream(), lines, used, comments)?;
            }
        }
        Ok(())
    }
    scan(
        tokens,
        &source.lines().collect::<Vec<_>>(),
        &mut std::collections::BTreeSet::new(),
        &comments,
    )
}

pub fn package_policy(dir: &Path, manifest: &toml::Table) -> Check {
    let deps = manifest
        .get("dependencies")
        .and_then(|v| v.as_table())
        .ok_or("FFI dependencies")?;
    require(
        deps.len() == 2
            && deps.contains_key("qso-graph-interop")
            && deps.contains_key("serde_json"),
        "unreviewed FFI dependency",
    )?;
    for (name, spec) in deps {
        require(
            spec.get("package").is_none(),
            format!("FFI package alias: {name}"),
        )?;
    }
    require(
        !manifest.contains_key("build-dependencies")
            && !manifest.contains_key("dev-dependencies")
            && !manifest.contains_key("target"),
        "unreviewed FFI dependency section",
    )?;
    for path in files(dir)? {
        let relative = path.strip_prefix(dir).map_err(|e| e.to_string())?;
        if relative == Path::new("include/qso_graph_interop.h")
            || relative == Path::new("tests/smoke.c")
            || relative == Path::new("tests/header.cpp")
        {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        require(
            ![
                "c", "cc", "cpp", "cxx", "h", "hpp", "s", "asm", "o", "obj", "a", "lib", "so",
                "dll", "dylib",
            ]
            .contains(&ext.as_str()),
            format!("unapproved FFI native file: {}", path.display()),
        )?;
    }
    Ok(())
}

fn generated(root: &Path) -> Result<Vec<u8>, String> {
    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        cpp_compat: true,
        include_guard: Some("QSO_GRAPH_INTEROP_H".into()),
        header: Some("/* SPDX-License-Identifier: GPL-3.0-or-later */\n/* Generated by cbindgen 0.29.4. Regenerate: cargo xtask ffi-header --write */".into()),
        usize_is_size_t: true,
        documentation_style: cbindgen::DocumentationStyle::Doxy,
        ..cbindgen::Config::default()
    };
    let bindings = cbindgen::Builder::new()
        .with_crate(root.join("crates/qso-graph-interop-ffi"))
        .with_config(config)
        .generate()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    bindings.write(&mut bytes);
    Ok(bytes)
}
fn matches_header(expected: &[u8], actual: &[u8]) -> Check {
    require(
        expected == actual,
        "C header drift: run cargo xtask ffi-header --write",
    )
}
pub fn header(root: &Path, write: bool) -> Check {
    let bytes = generated(root)?;
    let path = root.join("crates/qso-graph-interop-ffi/include/qso_graph_interop.h");
    if write {
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    } else {
        matches_header(&bytes, &std::fs::read(path).map_err(|e| e.to_string())?)
    }
}

pub fn c_compile(root: &Path, cpp: bool) -> Check {
    std::fs::create_dir_all(root.join("target/verify")).map_err(|e| e.to_string())?;
    let tool = if cpp { "c++" } else { "cc" };
    let mut cmd = Command::new(tool);
    cmd.current_dir(root).args([
        "-Wall",
        "-Wextra",
        "-Werror",
        "-Icrates/qso-graph-interop-ffi/include",
    ]);
    if cpp {
        cmd.args([
            "-std=c++11",
            "-fsyntax-only",
            "crates/qso-graph-interop-ffi/tests/header.cpp",
        ]);
    } else {
        cmd.args([
            "-std=c11",
            "crates/qso-graph-interop-ffi/tests/smoke.c",
            "target/debug/libqso_graph_interop_ffi.a",
            "-o",
            "target/verify/ffi-smoke",
        ]);
        if cfg!(target_os = "linux") {
            cmd.args(["-ldl", "-lpthread", "-lm"]);
        }
    }
    let status = cmd.status().map_err(|e| format!("{tool}: {e}; install a system C/C++ compiler (Debian/Ubuntu: apt install build-essential; macOS: xcode-select --install)"))?;
    require(
        status.success(),
        format!("{tool} compilation failed: {status}"),
    )
}
pub fn c_run(root: &Path) -> Check {
    let status = Command::new(root.join("target/verify/ffi-smoke"))
        .current_dir(root)
        .status()
        .map_err(|e| e.to_string())?;
    require(status.success(), format!("C ABI smoke test: {status}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planted_ffi_unsafe_and_header_failures() {
        assert!(root_policy("#![deny(unsafe_op_in_unsafe_fn)]").is_ok());
        assert!(root_policy("// #![deny(unsafe_op_in_unsafe_fn)]").is_err());
        assert!(root_policy("#![deny(unsafe_op_in_unsafe_fn)] #![cfg(any())]").is_err());
        assert!(unsafe_comments("// SAFETY: live pointer\nunsafe { read(); }").is_ok());
        for source in [
            "unsafe {}",
            "#[cfg(any())] unsafe {}",
            "let s = \"// SAFETY: fake\";\nunsafe {}",
            "// SAFETY: unrelated\nfn f() {}\nunsafe {}",
            "// SAFETY: outer only\nunsafe { unsafe {} }",
            "let s = r#\"\n// SAFETY: fake in raw string\n\"#; unsafe {}",
            "/*\n// SAFETY: fake in block comment\n*/ unsafe {}",
        ] {
            assert!(unsafe_comments(source).is_err(), "{source}");
        }
        assert!(matches_header(b"generated", b"generated").is_ok());
        assert!(matches_header(b"generated", b"planted drift").is_err());
        // The exception never weakens scanning for any other package.
        assert!(crate::source_tokens("unsafe {}".parse().unwrap()).is_err());
        assert!(crate::source_tokens_mode("macro_rules! m {}".parse().unwrap(), true).is_err());
    }
    #[test]
    fn checked_in_header_matches_code() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        header(root, false).unwrap();
        let expected = generated(root).unwrap();
        let mut planted = expected.clone();
        planted.extend_from_slice(b"/* planted header drift */");
        assert!(matches_header(&expected, &planted).is_err());
        assert!(
            crate::read(&root.join("crates/qso-graph-interop-ffi/include/qso_graph_interop.h"))
                .unwrap()
                .contains("qgi_decode_stream")
        );
    }
}
