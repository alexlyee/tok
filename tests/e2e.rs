// SPDX-FileCopyrightText: 2026 tok contributors
//
// SPDX-License-Identifier: MPL-2.0

use std::io::Write;
use std::process::{Command, Stdio};

fn tok_binary() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("tok");
    path
}

fn run_tok(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(tok_binary())
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("failed to execute tok");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

fn run_tok_stdin(input: &[u8], args: &[&str]) -> (String, String, i32) {
    let mut child = Command::new(tok_binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute tok");

    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

// 1. String mode
#[test]
fn t01_string_mode() {
    let (stdout, _, code) = run_tok(&["hello world"]);
    assert_eq!(code, 0);
    let count: usize = stdout.trim().parse().expect("should be integer");
    assert!(count > 0);
}

// 3. Stdin mode
#[test]
fn t03_stdin_mode() {
    let (stdout, _, code) = run_tok_stdin(b"hello world", &[]);
    assert_eq!(code, 0);
    let count: usize = stdout.trim().parse().expect("should be integer");
    assert!(count > 0);
}

// 15. Empty input
#[test]
fn t15_empty_input() {
    let (stdout, _, code) = run_tok(&[""]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "0");
}

// 18. No args, tty (simulate with empty piped stdin → prints 0)
#[test]
fn t18_no_args_empty_stdin() {
    let (_, stderr, code) = run_tok_stdin(b"", &[]);
    assert_eq!(code, 1, "empty stdin should exit 1");
    assert!(!stderr.is_empty(), "should print usage to stderr");
}
