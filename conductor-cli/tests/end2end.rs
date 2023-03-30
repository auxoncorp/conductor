use assert_cmd::prelude::*;
use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;
use std::process::Command;

/// ensure the `conductor` bin is fresh and build a `Command` for it
fn conductor_command() -> Command {
    Command::cargo_bin("conductor").expect("get conductor binary")
}

/// copy a tests system into a temporary directory and `cd` the command child to it
///
/// Note: Droppping the `TempDir` deletes the directory. Hold on to it until you're done.
fn unique_conductor(test_system_name: &str) -> (Command, TempDir) {
    let mut cmd = conductor_command();

    let test_system_dir = format!("../test_resources/systems/{test_system_name}");

    let dir = TempDir::new().unwrap();
    dir.copy_from(test_system_dir, &["*"]).unwrap();

    cmd.current_dir(&dir);

    (cmd, dir)
}

#[test]
fn exists() {
    conductor_command();
}

#[test]
fn can_run() {
    conductor_command().output().expect("run");
}

#[test]
fn bare_command_gives_help() {
    let mut cmd = conductor_command();

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn system_check_finds_right_config() {
    let (mut cmd, _context_dir) = unique_conductor("single-container-machine");

    cmd.args(["system", "check"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("container_system"));
}

#[test]
fn system_build_exits_successfully() {
    let (mut cmd, _context_dir) = unique_conductor("single-container-from-image");

    cmd.args(["system", "build"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("system built"));
}
