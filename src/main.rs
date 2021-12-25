use config::Config;
use error::TuxDriveResult;

use crate::watcher::Watcher;

mod config;
mod error;
mod watcher;

fn main() {
    use std::process::exit;

    if let Err(e) = setup_and_run() {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

const DEBOUNCE_TIME_IN_SECS: u64 = 10;

fn setup_and_run() -> TuxDriveResult<()> {
    let config = Config::read()?;
    let (mut watcher, file_events) = Watcher::<{ DEBOUNCE_TIME_IN_SECS }>::new()?;
    for path_conf in config.paths() {
        watcher.add_directory(path_conf.path(), path_conf.recursive())?;
    }
    Ok(())
}
