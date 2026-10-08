// SPDX-FileCopyrightText: 2026 tok contributors
//
// SPDX-License-Identifier: MPL-2.0

#[path = "counter.rs"]
mod counter;
use counter::*;

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "tok",
    version,
    about = "Count tokens in text",
    after_help = "\
EXAMPLES:
    tok \"hello world\"                     count tokens in a string
    tok myfile.md                         count tokens in a file
    echo \"text\" | tok                     count tokens from stdin

    A bare argument that looks like a filename (report.pdf) is treated as a
    path, so a typo errors instead of counting itself as text. To count such
    a string, pipe it: echo 'node.js' | tok"
)]
struct Cli {
    /// Text strings or file paths
    input: Vec<String>,
}

fn print_usage() {
    eprintln!("usage: tok [OPTIONS] <INPUT>...");
    eprintln!("       echo \"text\" | tok");
    eprintln!("       tok \"text to count\"");
    eprintln!("       tok path/to/file.md");
    eprintln!("\nRun 'tok --help' for full options.");
}

fn looks_like_path(s: &str) -> bool {
    if s.contains('/') || s.starts_with('.') || s.starts_with('~') {
        return true;
    }
    // a bare filename-looking argument (report.pdf, READMEE.md) is a path the
    // user typo'd, not a string to count: treating it as text returned a
    // bogus count with exit 0 — a silent wrong answer, the worst failure mode
    // "report.pdf" is a typo'd path; "3.14", "U.S.A" and "e.g" are text a
    // user wants counted. A bare token like "node.js" is genuinely ambiguous
    // — it is treated as a path, because a silent wrong count is worse than
    // an error a user can fix by quoting with a space or piping via stdin.
    // Require an extension that
    // looks like a real one (2+ chars, all alphabetic) AND a stem that is
    // not purely numeric or single-letter-dotted.
    if s.contains(' ') || s.len() >= 128 {
        return false;
    }
    let p = Path::new(s);
    let ext_ok = p
        .extension()
        .map(|e| {
            let e = e.to_string_lossy();
            (2..=8).contains(&e.len()) && e.chars().all(|c| c.is_ascii_alphabetic())
        })
        .unwrap_or(false);
    let stem_ok = p
        .file_stem()
        .map(|st| {
            let st = st.to_string_lossy();
            // "3" (from 3.14) or "U.S" (from U.S.A) are not filenames
            !st.is_empty()
                && !st.chars().all(|c| c.is_ascii_digit() || c == '.')
                && st.split('.').all(|part| part.len() != 1)
        })
        .unwrap_or(false);
    ext_ok && stem_ok
}

fn count_file(bpe: &tiktoken_rs::CoreBPE, path: &Path) -> usize {
    let content = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("tok: {}: {}", path.display(), e);
            return 0;
        }
    };
    let (text, had_non_utf8) = decode_bytes(&content);
    if had_non_utf8 {
        eprintln!(
            "tok: warning: {}: contains non-utf8 bytes (replaced with \u{FFFD})",
            path.display()
        );
    }
    count_tokens(bpe, &text)
}

fn decode_bytes(bytes: &[u8]) -> (String, bool) {
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => (s, false),
        Err(_) => {
            let text = String::from_utf8_lossy(bytes).into_owned();
            (text, true)
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let bpe = default_bpe();

    if cli.input.is_empty() {
        if io::stdin().is_terminal() {
            print_usage();
            process::exit(1);
        }
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("tok: failed to read stdin: {}", e);
            process::exit(1);
        });
        if buf.is_empty() {
            print_usage();
            process::exit(1);
        }
        println!("{}", count_tokens(&bpe, &buf));
        return;
    }

    let mut all_files: Vec<PathBuf> = Vec::new();
    let mut string_args: Vec<String> = Vec::new();

    for arg in &cli.input {
        let path = Path::new(arg);
        if path.is_file() {
            all_files.push(path.to_path_buf());
        } else if looks_like_path(arg) {
            eprintln!("tok: {}: No such file or directory", arg);
            process::exit(1);
        } else {
            string_args.push(arg.clone());
        }
    }

    if !all_files.is_empty() && !string_args.is_empty() {
        eprintln!("tok: cannot mix file paths and string arguments");
        process::exit(1);
    }

    if !string_args.is_empty() {
        let combined = string_args.join(" ");
        println!("{}", count_tokens(&bpe, &combined));
        return;
    }

    let total: usize = all_files.iter().map(|p| count_file(&bpe, p)).sum();
    println!("{}", total);
}
