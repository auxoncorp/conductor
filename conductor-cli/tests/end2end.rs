use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn conductor_command() -> Command {
    Command::cargo_bin("conductor").expect("get conductor binary")
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
    let mut cmd = conductor_command();

    cmd.args(["system", "check"]);
    cmd.current_dir("../test_resources/systems/single-docker-machine/");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("container system"));
}
