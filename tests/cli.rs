//! Command line tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn fail_config_file_unknown_extension() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]);
    cmd.arg("--config").arg("config.unknown");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("extension"));

    Ok(())
}

#[test]
fn fail_config_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]);
    cmd.arg("--config").arg("not_found.json");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("os error 2"));

    Ok(())
}

#[test]
fn ignore() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child(".gitignore").write_str("hidden.md")?;
    dir.child("visible.md").write_str("Visible")?;
    dir.child("hidden.md").write_str("Hidden")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/visible/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Visible"));

    dir.child("_site/hidden/index.html")
        .assert(predicate::path::exists().not());

    Ok(())
}

#[test]
fn typescript() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("index.html")
        .write_str(r#"<script src="script.ts"></script>"#)?;

    dir.child("script.ts").write_str(
        r#"// Comment
const myVar: string = "abc";
console.log(myVar);
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("script.js"));

    dir.child("_site/script.js")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("string").not())
        .assert(predicate::str::contains("myVar"))
        .assert(predicate::str::contains("abc"));

    Ok(())
}

#[test]
fn javascript() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("index.html")
        .write_str(r#"<script src="script.js">"#)?;

    dir.child("script.js").write_str(
        r#"// Comment
alert('Hello, World!');
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("script.js"));

    dir.child("_site/script.js")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Comment").not())
        .assert(predicate::str::contains("alert"))
        .assert(predicate::str::contains("Hello, World!"));

    Ok(())
}

#[test]
fn markdown() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("index.md").write_str("*Italic*")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("<em>Italic</em>"));

    Ok(())
}

#[test]
fn scss() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("index.html")
        .write_str(r#"<link rel="stylesheet" href="style.scss">"#)?;

    dir.child("style.scss").write_str(
        r#"// Comment
body {
  main {
    margin: 0;
  }
}
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("style.css"));

    dir.child("_site/style.css")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Comment").not())
        .assert(predicate::str::contains("body main"))
        .assert(predicate::str::contains("margin"));

    Ok(())
}

#[test]
fn css() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("index.html")
        .write_str(r#"<link rel="stylesheet" href="style.css">"#)?;

    dir.child("style.css").write_str(
        r#"/* Comment */
body {
  margin: 0;
}
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("style.css"));

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
layout: page.jinja
---
# About
"#,
    )?;

    dir.child("_layouts/page.jinja").write_str(
        r#"<!DOCTYPE html>
<title>{{ page.title }}</title>
<body>{{ content | safe }}</body>
"#,
    )?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert().success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("About"))
        .assert(predicate::str::contains("Blog"));

    Ok(())
}
