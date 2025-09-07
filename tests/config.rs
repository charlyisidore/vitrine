//! Config tests.

use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn config_not_found() -> Result<()> {
    Command::cargo_bin("vitrine")?
        .args(["build", "--config", "not_found.ts"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("os error 2"));

    Ok(())
}

#[test]
fn config_detect_ts() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    dir.child("vitrine.config.ts").write_str(
        r#"
            export default {
                input_dir: "foo",
                output_dir: "bar",
            };
        "#,
    )?;
    dir.child("foo/index.md").write_str("foo")?;

    Command::cargo_bin("vitrine")?
        .args(["build"])
        .current_dir(&dir)
        .assert()
        .success();

    dir.child("bar/index.html")
        .assert(predicate::path::is_file());

    Ok(())
}

#[test]
fn config_detect_js() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    dir.child("vitrine.config.js").write_str(
        r#"
            export default {
                input_dir: "foo",
                output_dir: "bar",
            };
        "#,
    )?;
    dir.child("foo/index.md").write_str("foo")?;

    Command::cargo_bin("vitrine")?
        .args(["build"])
        .current_dir(&dir)
        .assert()
        .success();

    dir.child("bar/index.html")
        .assert(predicate::path::is_file());

    Ok(())
}
