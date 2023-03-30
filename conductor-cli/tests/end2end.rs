use assert_cmd::prelude::*;
use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;
use std::ffi::OsStr;
use std::process::Command;

/// ensure the `conductor` bin is fresh and build a `Command` for it
fn conductor_command() -> Command {
    Command::cargo_bin("conductor").expect("get conductor binary")
}

/// Create a copy of a test system in a temporary directory, build `Commands` for `conductor` `cd`d
/// into that directory.
///
/// Note: Droppping this deletes the temporary directory. Hold on to it until you're done.
struct UniqueConductor {
    tmp: TempDir,
}

impl UniqueConductor {
    fn new(test_system_name: &str) -> Self {
        let test_system_dir = format!("../test_resources/systems/{test_system_name}");

        let dir = TempDir::new().unwrap();
        dir.copy_from(test_system_dir, &["*"]).unwrap();

        Self { tmp: dir }
    }

    fn cmd<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = conductor_command();

        cmd.current_dir(&self.tmp);
        cmd.args(args);

        cmd
    }
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
    let cond = UniqueConductor::new("single-container-machine");

    let mut cmd = cond.cmd(["system", "check"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("container_system"));
}

#[test]
fn system_build_exits_successfully() {
    let cond = UniqueConductor::new("single-container-machine");

    let mut cmd = cond.cmd(["system", "build"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("system built"));
}

#[test]
fn system_builds_and_starts() {
    let cond = UniqueConductor::new("single-container-machine");

    let mut cmd = cond.cmd(["system", "build"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("system built"));
}

#[test]
fn system_with_ext_bin_builds_and_starts() {
    let cond = UniqueConductor::new("single-container-machine");

    // build
    let mut build = cond.cmd(["system", "build"]);

    build
        .assert()
        .success()
        .stdout(predicate::str::contains("system built"));

    // start
    let mut start = cond.cmd(["system", "start"]);

    start
        .assert()
        .success()
        .stdout(predicate::str::contains("system started"));

    //TODO: make sure that the right thing got started (spoiler: it's didn't)
}
