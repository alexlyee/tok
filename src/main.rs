// SPDX-FileCopyrightText: 2026 tok contributors
//
// SPDX-License-Identifier: MPL-2.0

#[path = "counter.rs"]
mod counter;
use counter::*;

use std::fs::{self, File};
use std::io::{self, BufReader, IsTerminal, Read};
use std::path::{Component, Path, PathBuf};
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
    tok ./notes/                          count .md and .txt files recursively
    tok -xrs,toml ./src/                  count .rs and .toml files
    tok -a ./project/                     count all text files
    tok -aX'target/**' ./project/         exclude target/ on top of defaults

    A bare argument that looks like a filename (report.pdf) is treated as a
    path, so a typo errors instead of counting itself as text. To count such
    a string, pipe it: echo 'node.js' | tok"
)]
struct Cli {
    /// Text strings, file paths, directories, or glob patterns
    input: Vec<String>,

    /// File extensions to include in directory mode (without dot, repeatable). Default: md, txt
    #[arg(short = 'x', long = "ext")]
    ext: Vec<String>,

    /// Include all text files in directory mode (skip binary). Overrides --ext
    #[arg(short, long)]
    all: bool,

    /// Gitignore-style glob to exclude (repeatable)
    #[arg(short = 'X', long = "exclude")]
    exclude: Vec<String>,

    /// Disable default exclusions (node_modules, components starting with . or _)
    #[arg(short = 'A', long = "no-default-excludes")]
    no_default_excludes: bool,
}

fn print_usage() {
    eprintln!("usage: tok [OPTIONS] <INPUT>...");
    eprintln!("       echo \"text\" | tok");
    eprintln!("       tok \"text to count\"");
    eprintln!("       tok path/to/file.md");
    eprintln!("       tok file1.md file2.md");
    eprintln!("       tok ./directory/");
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

fn has_glob_meta(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn is_binary(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 8192];
    let Ok(n) = io::Read::read(&mut BufReader::new(file), &mut buf) else {
        return false;
    };
    buf[..n].contains(&0)
}

fn default_excluded(rel: &Path) -> bool {
    for comp in rel.components() {
        if let Component::Normal(c) = comp {
            let s = c.to_string_lossy();
            if s == "node_modules" {
                return true;
            }
            if s.starts_with('.') || s.starts_with('_') {
                return true;
            }
        }
    }
    false
}

fn matches_any_glob(rel: &Path, patterns: &[glob::Pattern]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let s = rel.to_string_lossy();
    let opts = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    patterns.iter().any(|p| {
        p.matches_with(&s, opts)
            || rel
                .file_name()
                .map(|n| p.matches_with(&n.to_string_lossy(), opts))
                .unwrap_or(false)
    })
}

fn collect_dir_files(root: &Path, cli: &Cli, excludes: &[glob::Pattern]) -> Vec<PathBuf> {
    let extensions: Vec<String> = if cli.all {
        vec![]
    } else if cli.ext.is_empty() {
        vec!["md".to_string(), "txt".to_string()]
    } else {
        cli.ext
            .iter()
            .flat_map(|e| e.split(','))
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty())
            .collect()
    };

    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let rel = e.path().strip_prefix(root).unwrap_or(e.path());
            if !cli.no_default_excludes && default_excluded(rel) {
                return false;
            }
            !matches_any_glob(rel, excludes)
        })
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|path| {
            if cli.all {
                !is_binary(path)
            } else {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                extensions.iter().any(|e| *e == ext)
            }
        })
        .collect();
    files.sort();
    files
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

    let exclude_patterns: Vec<glob::Pattern> = cli
        .exclude
        .iter()
        .filter_map(|s| match glob::Pattern::new(s) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("tok: invalid exclude '{}': {}", s, e);
                None
            }
        })
        .collect();

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
        if path.is_dir() {
            let files = collect_dir_files(path, &cli, &exclude_patterns);
            if files.is_empty() {
                eprintln!("tok: {}: no matching files found", arg);
            }
            all_files.extend(files);
        } else if path.is_file() {
            all_files.push(path.to_path_buf());
        } else if has_glob_meta(arg) {
            match glob::glob(arg) {
                Ok(paths) => {
                    let matched: Vec<PathBuf> = paths.filter_map(|p| p.ok()).collect();
                    if matched.is_empty() {
                        eprintln!("tok: note: glob '{}' matched no files, treating as string", arg);
                        string_args.push(arg.clone());
                    } else {
                        all_files.extend(matched);
                    }
                }
                Err(e) => {
                    eprintln!("tok: invalid glob '{}': {}", arg, e);
                    process::exit(1);
                }
            }
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
