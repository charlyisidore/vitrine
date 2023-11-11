//! Command line tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn unknown_config_file_extension() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.arg("--config").arg("config.unknown");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("extension"));

    Ok(())
}

#[test]
fn config_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.arg("--config").arg("not_found.json");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file"));

    Ok(())
}

#[test]
fn zero_config() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?.into_persistent();

    dir.child("index.md").write_str(
        r#"---
title: Blog
layout: page.tera
---
# About
"#,
    )?;

    dir.child("_data/meta.json")
        .write_str(r#"{ "author": "Doe" }"#)?;

    dir.child("_layouts/page.tera").write_str(
        r#"
<!DOCTYPE html>
<meta name="author" content="{{ meta.author }}" />
<title>{{ title }}</title>
<body>{{ content | safe }}</body>
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("About"))
        .assert(predicate::str::contains("Blog"))
        .assert(predicate::str::contains("Doe"));

    Ok(())
}
