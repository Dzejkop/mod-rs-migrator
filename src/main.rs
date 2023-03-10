use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
struct Args {
    #[clap(flatten)]
    config: Config,

    target: PathBuf,
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
struct Config {
    #[clap(short, long)]
    follow_symlinks: bool,

    #[clap(short, long)]
    leave_empty_dirs: bool,
}

async fn find_mod_named_modules<F>(
    path: impl AsRef<Path>,
    filter: F,
    config: &Config,
) -> anyhow::Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
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

                if filter(&p) {
                    results.push(p);
                }
            }
        }
    }

    Ok(results)
}

async fn move_mod_rs_outside_of_dir(
    mod_files: Vec<PathBuf>,
    config: &Config,
) -> anyhow::Result<()> {
    for mod_file in mod_files {
        let parent_dir = mod_file
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Missing parent"))?;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let progress_bar = ProgressBar::new(1)
        .with_style(ProgressStyle::default_spinner())
        .with_message("Looking for mod.rs files");

    let counter = AtomicUsize::new(0);

    let mod_files = find_mod_named_modules(
        args.target,
        |p| {
            if p.file_name() == Some(OsStr::new("mod.rs")) {
                let n = counter.fetch_add(1, Ordering::SeqCst);

                progress_bar.set_message(format!("Found {n} mod.rs files"));

                true
            } else {
                false
            }
        },
        &args.config,
    )
    .await?;

    progress_bar.set_message("Moving mod files");

    move_mod_rs_outside_of_dir(mod_files, &args.config).await?;

    progress_bar.finish();

    Ok(())
}
