use std::path::PathBuf;
use std::process::Command;

const FIXTURE: &str = "tests/fixtures/r_adapter/sample.R";

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ast-bro"))
}

fn run(args: &[&str]) -> String {
    let output = Command::new(bin())
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run");
    assert!(output.status.success(), "exit non-zero: {output:?}");
    String::from_utf8(output.stdout).expect("utf8")
}

#[test]
fn functions_and_fields_render() {
    let output = run(&["map", FIXTURE]);
    assert!(
        output.contains("normalize <- function(values, center = TRUE)"),
        "function missing:\n{output}"
    );
    assert!(
        output.contains("summarize = \\(values)"),
        "short lambda missing:\n{output}"
    );
    assert!(output.contains("DEFAULT_LIMIT"), "field missing:\n{output}");
}

#[test]
fn show_extracts_r_function() {
    let output = run(&["show", FIXTURE, "normalize"]);
    assert!(
        output.contains("stats::sd(values)"),
        "body missing:\n{output}"
    );
}

#[test]
fn run_uses_r_pattern_preprocessing() {
    let output = run(&[
        "run",
        "-p",
        "$NAME <- function($ARG, $$$REST) $BODY",
        "-l",
        "r",
        FIXTURE,
    ]);
    assert!(
        output.contains("normalize <- function"),
        "R pattern did not match:\n{output}"
    );
}
