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

// 2. File mode
#[test]
fn t02_file_mode() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("input.txt");
    std::fs::write(&f, "hello world").unwrap();

    let (stdout, _, code) = run_tok(&[f.to_str().unwrap()]);
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

// 4. Consistency across modes
#[test]
fn t04_consistency() {
    let text = "The quick brown fox jumps over the lazy dog.";
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("fox.txt");
    std::fs::write(&f, text).unwrap();

    let (s_out, _, _) = run_tok(&[text]);
    let (f_out, _, _) = run_tok(&[f.to_str().unwrap()]);
    let (p_out, _, _) = run_tok_stdin(text.as_bytes(), &[]);

    let s: usize = s_out.trim().parse().unwrap();
    let fv: usize = f_out.trim().parse().unwrap();
    let p: usize = p_out.trim().parse().unwrap();

    assert_eq!(s, fv, "string vs file");
    assert_eq!(s, p, "string vs stdin");
}

// 5. File-vs-string disambiguation
#[test]
fn t05_file_vs_string_disambiguation() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.txt");
    std::fs::write(&f, "this file has several tokens in it for testing").unwrap();

    let (file_out, _, _) = run_tok(&[f.to_str().unwrap()]);
    let file_count: usize = file_out.trim().parse().unwrap();

    let (lit_out, _, _) = run_tok_stdin(b"test.txt", &[]);
    let lit_count: usize = lit_out.trim().parse().unwrap();

    assert_ne!(file_count, lit_count, "should count file contents, not filename string");
}

// 15. Empty input
#[test]
fn t15_empty_input() {
    let (stdout, _, code) = run_tok(&[""]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "0");
}

// 17. Missing file
#[test]
fn t17_missing_file() {
    let (_, stderr, code) = run_tok(&["/nonexistent/path/to/file.txt"]);
    assert_eq!(code, 1);
    assert!(!stderr.is_empty());
}

// 18. No args, tty (simulate with empty piped stdin → prints 0)
#[test]
fn t18_no_args_empty_stdin() {
    let (_, stderr, code) = run_tok_stdin(b"", &[]);
    assert_eq!(code, 1, "empty stdin should exit 1");
    assert!(!stderr.is_empty(), "should print usage to stderr");
}

// 19. Large input
#[test]
fn t19_large_input() {
    let large = "abcdefghij ".repeat(100_000);
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("large.txt");
    std::fs::write(&f, &large).unwrap();

    let (stdout, _, code) = run_tok(&[f.to_str().unwrap()]);
    assert_eq!(code, 0);
    let count: usize = stdout.trim().parse().expect("should be integer");
    assert!(count > 1000);
}
