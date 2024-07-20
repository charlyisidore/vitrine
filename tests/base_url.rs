//! Base URL tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn empty() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.json").write_str(
        r#"
        {
            "input_dir": ".",
            "output_dir": "_dist",
            "base_url": ""
        }
        "#,
    )?;

    dir.child("index.md").write_str("Home [link](foo/bar.md)")?;
    dir.child("foo/bar.md")
        .write_str("FooBar [link](../index.md)")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);
    cmd.assert().success();

    dir.child("_dist/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"))
        .assert(predicate::str::contains(r#"<a href=/foo/bar>link</a>"#));

    dir.child("_dist/foo/bar/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("FooBar"))
        .assert(predicate::str::contains(r#"<a href=/>link</a>"#));

    Ok(())
}

#[test]
fn path() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.json").write_str(
        r#"
        {
            "input_dir": ".",
            "output_dir": "_dist",
            "base_url": "/my/demo"
        }
        "#,
    )?;

    dir.child("index.md").write_str("Home [link](foo/bar.md)")?;
    dir.child("foo/bar.md")
        .write_str("FooBar [link](../index.md)")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);
    cmd.assert().success();

    dir.child("_dist/my/demo/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"))
        .assert(predicate::str::contains(
            r#"<a href=/my/demo/foo/bar>link</a>"#,
        ));

    dir.child("_dist/my/demo/foo/bar/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("FooBar"))
        .assert(predicate::str::contains(r#"<a href=/my/demo/>link</a>"#));

    Ok(())
}

#[test]
fn url() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.json").write_str(
        r#"
        {
            "input_dir": ".",
            "output_dir": "_dist",
            "base_url": "https://example.com/my/demo"
        }
        "#,
    )?;

    dir.child("index.md").write_str("Home [link](foo/bar.md)")?;
    dir.child("foo/bar.md")
        .write_str("FooBar [link](../index.md)")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);
    cmd.assert().success();

    dir.child("_dist/my/demo/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"))
        .assert(predicate::str::contains(
            r#"<a href=/my/demo/foo/bar>link</a>"#,
        ));

    dir.child("_dist/my/demo/foo/bar/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("FooBar"))
        .assert(predicate::str::contains(r#"<a href=/my/demo/>link</a>"#));

    Ok(())
}
