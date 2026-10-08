// SPDX-FileCopyrightText: 2026 tok contributors
//
// SPDX-License-Identifier: MPL-2.0

#[path = "counter.rs"]
mod counter;
use counter::*;

use std::io::{self, IsTerminal, Read};
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
    echo \"text\" | tok                     count tokens from stdin"
)]
struct Cli {
    /// Text strings to count
    input: Vec<String>,
}

fn print_usage() {
    eprintln!("usage: tok [OPTIONS] <INPUT>...");
    eprintln!("       echo \"text\" | tok");
    eprintln!("       tok \"text to count\"");
    eprintln!("\nRun 'tok --help' for full options.");
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

    let combined = cli.input.join(" ");
    println!("{}", count_tokens(&bpe, &combined));
}
