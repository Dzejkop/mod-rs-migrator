use std::path::PathBuf;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use mod_rs_migrator::{find_mod_named_modules, move_mod_rs_outside_of_dir, Config};

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
struct Args {
    #[clap(flatten)]
    config: Config,

    target: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let progress_bar = ProgressBar::new(1)
        .with_style(ProgressStyle::default_spinner())
        .with_message("Looking for mod.rs files");

    let mod_files = find_mod_named_modules(args.target, &args.config).await?;

    progress_bar.set_message(format!("Moving {} mod files", mod_files.len()));

    move_mod_rs_outside_of_dir(mod_files, &args.config).await?;

    progress_bar.finish();

    Ok(())
}
