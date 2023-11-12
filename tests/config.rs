//! Configuration loading tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn config_default_json() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.json").write_str(
        r#"{
  "input_dir": "content",
  "output_dir": "public"
}
"#,
    )?;

    dir.child("content/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.json"));

    dir.child("public/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn config_default_lua() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.lua").write_str(
        r#"-- Config
return {
    input_dir = "content",
    output_dir = "public",
}
"#,
    )?;

    dir.child("content/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.lua"));

    dir.child("public/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn config_default_rhai() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.rhai").write_str(
        r#"// Config
#{
    input_dir: "content",
    output_dir: "public",
}
"#,
    )?;

    dir.child("content/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.rhai"));

    dir.child("public/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn config_default_toml() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.toml").write_str(
        r#"# Config
input_dir = "content"
output_dir = "public"
"#,
    )?;

    dir.child("content/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.toml"));

    dir.child("public/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn config_default_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.yaml").write_str(
        r#"# Config
input_dir: content
output_dir: public
"#,
    )?;

    dir.child("content/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.yaml"));

    dir.child("public/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}
