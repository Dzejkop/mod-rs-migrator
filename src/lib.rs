use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Debug, Default, Clone, Parser)]
#[clap(rename_all = "kebab-case")]
pub struct Config {
    /// If set will follow symlinks as if they were directories - I don't recommend using this
    #[clap(short, long)]
    follow_symlinks: bool,

    /// If set will not delete directories that would end up empty after a run
    #[clap(short, long)]
    leave_empty_dirs: bool,

    /// If set will not apply special treatment to directories named `tests`
    ///
    /// By default, if a `mod.rs` file would be moved to a directory named `tests` it will be preserved as a mod.rs file
    #[clap(short, long)]
    no_special_treatment_for_tests_dir: bool,
}

pub async fn find_mod_named_modules(
    path: impl AsRef<Path>,
    config: &Config,
) -> anyhow::Result<Vec<PathBuf>> {
    let path = path.as_ref();

    let mut dirs_to_process = vec![path.to_owned()];
    let mut results = vec![];

    while let Some(dir) = dirs_to_process.pop() {
        let mut read_dir = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                dirs_to_process.push(entry.path());
            }

            if config.follow_symlinks && file_type.is_symlink() {
                dirs_to_process.push(entry.path());
            }

            if file_type.is_file() {
                let p = entry.path();

                if p.file_name() == Some(OsStr::new("mod.rs")) {
                    results.push(p);
                }
            }
        }
    }

    Ok(results)
}

pub async fn move_mod_rs_outside_of_dir(
    mod_files: Vec<PathBuf>,
    config: &Config,
) -> anyhow::Result<()> {
    for mod_file in mod_files {
        let parent_dir = mod_file
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Missing parent"))?;

        if !config.no_special_treatment_for_tests_dir {
            let parent_of_parent = parent_dir
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Missing parent"))?;

            if parent_of_parent.file_name() == Some(OsStr::new("tests")) {
                continue;
            }
        }

        let new_path = parent_dir.with_extension("rs");

        move_file(&mod_file, new_path).await?;

        if !config.leave_empty_dirs && is_dir_empty(parent_dir).await? {
            tokio::fs::remove_dir(parent_dir).await?;
        }
    }

    Ok(())
}

async fn is_dir_empty(dir: impl AsRef<Path>) -> anyhow::Result<bool> {
    let mut read_dir = tokio::fs::read_dir(dir).await?;

    Ok(read_dir.next_entry().await?.is_none())
}

async fn move_file(
    source_path: impl AsRef<Path>,
    target_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let source_path = source_path.as_ref();

    tokio::fs::copy(source_path, target_path).await?;

    tokio::fs::remove_file(source_path).await?;

    Ok(())
}
