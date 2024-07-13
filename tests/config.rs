//! Configuration tests.

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn default_js() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.js").write_str(
        r#"// Config
            export default {
                input_dir: "my_input",
                output_dir: "my_output",
            };
        "#,
    )?;

    dir.child("my_input/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.js"));

    dir.child("my_output/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn default_json() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.json").write_str(
        r#"
        {
            "input_dir": "my_input",
            "output_dir": "my_output"
        }
        "#,
    )?;

    dir.child("my_input/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.json"));

    dir.child("my_output/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn default_lua() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.lua").write_str(
        r#"-- Config
        return {
            input_dir = "my_input",
            output_dir = "my_output",
        }
        "#,
    )?;

    dir.child("my_input/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.lua"));

    dir.child("my_output/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn default_rhai() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.rhai").write_str(
        r#"// Config
        #{
            input_dir: "my_input",
            output_dir: "my_output",
        }
        "#,
    )?;

    dir.child("my_input/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.rhai"));

    dir.child("my_output/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn default_toml() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.toml").write_str(
        r#"# Config
        input_dir = "my_input"
        output_dir = "my_output"
        "#,
    )?;

    dir.child("my_input/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.toml"));

    dir.child("my_output/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}

#[test]
fn default_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let dir = assert_fs::TempDir::new()?;

    dir.child("vitrine.config.yaml").write_str(
        r#"# Config
        input_dir: my_input
        output_dir: my_output
        "#,
    )?;

    dir.child("my_input/index.md").write_str("Home")?;

    let mut cmd = Command::cargo_bin("vitrine")?;
    cmd.args(["build"]).current_dir(&dir);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vitrine.config.yaml"));

    dir.child("my_output/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("Home"));

    Ok(())
}
