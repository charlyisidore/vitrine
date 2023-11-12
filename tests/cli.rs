//! Command line tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn fail_config_file_unknown_extension() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.arg("--config").arg("config.unknown");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("extension"));

    Ok(())
}

#[test]
fn fail_config_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.arg("--config").arg("not_found.json");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file"));

    Ok(())
}

#[test]
fn javascript() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("script.js").write_str(
        r#"// Comment
document.addEventListener('DOMContentLoaded', () => {
    alert('Hello, World!');
});
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/script.js")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Comment").not())
        .assert(predicate::str::contains("addEventListener"))
        .assert(predicate::str::contains("DOMContentLoaded"))
        .assert(predicate::str::contains("Hello, World!"));

    Ok(())
}

#[test]
fn stylesheet() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("style.css").write_str(
        r#"/* Comment */
body {
  margin: 0;
}
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/style.css")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Comment").not())
        .assert(predicate::str::contains("body"))
        .assert(predicate::str::contains("margin"));

    Ok(())
}

#[test]
fn url() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    // /
    dir.child("index.md").write_str("Home")?;

    // /blog
    dir.child("blog/index.md").write_str("Blog")?;

    // /blog/1970-01-01
    dir.child("blog/1970-01-01.md").write_str("Unix")?;

    // /custom/url
    dir.child("custom.md").write_str(
        r#"---
url: /custom/url
---
Custom
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

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

    dir.child("_site/custom/url/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Custom"));

    Ok(())
}

#[test]
fn zero_config() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

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
        r#"<!DOCTYPE html>
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
