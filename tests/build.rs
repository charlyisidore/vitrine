//! Build tests.

use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn ignore_paths() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    dir.child("vitrine.config.ts").write_str(
        r#"
            export default {
                ignore_paths: ["hidden-config.md"],
            };
        "#,
    )?;
    dir.child(".git").create_dir_all()?;
    dir.child(".gitignore").write_str("hidden-git.md")?;
    dir.child("visible.md").write_str("foo")?;
    dir.child("hidden-config.md").write_str("config")?;
    dir.child("hidden-git.md").write_str("git")?;
    dir.child("_hidden-underscore.md").write_str("underscore")?;

    Command::cargo_bin("vitrine")?
        .args(["build", "--config", "vitrine.config.ts"])
        .current_dir(&dir)
        .assert()
        .success();

    dir.child("_site/visible/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("foo"));
    dir.child("_site/hidden-config/index.html")
        .assert(predicate::path::exists().not());
    dir.child("_site/hidden-git/index.html")
        .assert(predicate::path::exists().not());
    dir.child("_site/_hidden-underscore/index.html")
        .assert(predicate::path::exists().not());

    Ok(())
}

#[test]
fn url() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    dir.child("index.md").write_str("home")?;
    dir.child("foo/index.md").write_str("foo")?;
    dir.child("foo/bar.md").write_str("bar")?;
    dir.child("baz.md").write_str(
        r#"---
url: /foo/baz
---
baz"#,
    )?;

    Command::cargo_bin("vitrine")?
        .args(["build"])
        .current_dir(&dir)
        .assert()
        .success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("home"));
    dir.child("_site/foo/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("foo"));
    dir.child("_site/foo/bar/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("bar"));
    dir.child("_site/foo/baz/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("baz"));

    Ok(())
}

#[test]
fn markdown() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    dir.child("index.md").write_str("*Italic*")?;

    Command::cargo_bin("vitrine")?
        .args(["build"])
        .current_dir(&dir)
        .assert()
        .success();

    dir.child("_site/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("<em>Italic</em>"));

    Ok(())
}

#[test]
fn assets() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    dir.child("foo.md").write_str(
        r#"---
layout: page.jinja
---
foo"#,
    )?;
    dir.child("_layouts/page.jinja").write_str(
        r#"
            <html>
                <head>
                    <link rel="stylesheet" href="/assets/style.scss">
                    <script src="/assets/script.ts"></script>
                </head>
                <body></body>
            </html>
        "#,
    )?;
    dir.child("assets/script.ts")
        .write_str(r#"console.log("foo");"#)?;
    dir.child("assets/style.scss")
        .write_str(r#"body { color: red; }"#)?;

    Command::cargo_bin("vitrine")?
        .args(["build"])
        .current_dir(&dir)
        .assert()
        .success();

    dir.child("_site/foo/index.html")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("/assets/script.js"))
        .assert(predicate::str::contains("/assets/style.css"));
    dir.child("_site/assets/script.js")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("console.log"))
        .assert(predicate::str::contains("foo"));
    dir.child("_site/assets/style.css")
        .assert(predicate::path::is_file())
        .assert(predicate::str::contains("body"))
        .assert(predicate::str::contains("color"));

    Ok(())
}
