//! URL tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn from_input() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    // /
    dir.child("index.md").write_str("Home")?;

    // /blog
    dir.child("blog/index.md").write_str("Blog")?;

    // /blog/1970-01-01
    dir.child("blog/1970-01-01.md").write_str("Unix")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);
    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    dir.child("_site/blog/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Blog"));

    dir.child("_site/blog/1970-01-01/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Unix"));

    Ok(())
}

#[test]
fn custom_relative() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("custom-relative.md").write_str(
        r#"---
url: /my/demo
---
Custom
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);
    cmd.assert().success();

    dir.child("_site/my/demo/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Custom"));

    Ok(())
}

#[test]
fn custom_absolute() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("custom-absolute.md").write_str(
        r#"---
url: https://example.com/my/demo
---
Custom
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);
    cmd.assert().success();

    dir.child("_site/my/demo/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Custom"));

    Ok(())
}
