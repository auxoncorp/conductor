use anyhow::Result;
use assert_cmd::prelude::*;
use std::process::Command;
use std::str;

#[test]
fn exists() -> Result<()> {
    Command::cargo_bin("conductor")?;

    Ok(())
}

#[test]
fn can_run() -> Result<()> {
    Command::cargo_bin("conductor")?.output()?;

    Ok(())
}

#[test]
fn bare_command_gives_help() -> Result<()> {
    let out = Command::cargo_bin("conductor")?.output()?;

    assert!(!out.status.success(), "bare command didn't exit with fail");
    assert!(
        str::from_utf8(&out.stderr)?.contains("Usage:"),
        "bare command didn't show help"
    );

    Ok(())
}
