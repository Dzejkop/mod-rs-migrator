use std::path::Path;
use std::process::Stdio;

use indoc::indoc;
use mod_rs_migrator::Config;
use tempdir::TempDir;
use tokio::fs::{self, File};
use tokio::process::Command;

const EXP_AFTER_CREATION: &str = indoc! {r#"
.:
src
tests

./src:
a
b
lib.rs

./src/a:
c
mod.rs

./src/a/c:
mod.rs

./src/b:
mod.rs

./tests:
a
basic.rs

./tests/a:
mod.rs
something.rs
"#};

const EXP_AFTER_RUN: &str = indoc! {r#"
.:
src
tests

./src:
a
a.rs
b.rs
lib.rs

./src/a:
c.rs

./tests:
a
basic.rs

./tests/a:
mod.rs
something.rs
"#};

#[tokio::test]
async fn integration() -> anyhow::Result<()> {
    let dir = TempDir::new("mod-rs-migrator")?;

    prepare_test_dir(dir.path()).await?;

    let output = ls_recursive(dir.path()).await?;

    assert_eq!(output, EXP_AFTER_CREATION);

    let config = Config::default();
    let files = mod_rs_migrator::find_mod_named_modules(dir.path(), &config).await?;
    mod_rs_migrator::move_mod_rs_outside_of_dir(files, &config).await?;

    let output = ls_recursive(dir.path()).await?;

    assert_eq!(output, EXP_AFTER_RUN);

    dir.close()?;

    Ok(())
}

async fn prepare_test_dir(dir: impl AsRef<Path>) -> anyhow::Result<()> {
    let dir = dir.as_ref();

    let src = dir.join("src");
    let src_lib = src.join("lib.rs");
    let src_a_mod = src.join("a").join("mod.rs");
    let src_a_c_mod = src.join("a").join("c").join("mod.rs");
    let src_b_mod = src.join("b").join("mod.rs");

    let tests = dir.join("tests");
    let tests_basic = tests.join("basic.rs");
    let tests_a_mod = tests.join("a").join("mod.rs");
    let tests_a_something = tests.join("a").join("something.rs");

    let all_files = vec![
        &src_lib,
        &src_a_mod,
        &src_a_c_mod,
        &src_b_mod,
        &tests_basic,
        &tests_a_mod,
        &tests_a_something,
    ];

    for file in all_files {
        fs::create_dir_all(file.parent().unwrap()).await?;
        File::create(file).await?;
    }

    Ok(())
}

async fn ls_recursive(path: impl AsRef<Path>) -> anyhow::Result<String> {
    let path = path.as_ref();

    let output = Command::new("ls")
        .current_dir(path)
        .arg("-R")
        .arg(".")
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    Ok(String::from_utf8(output.stdout)?)
}
