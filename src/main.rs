use config::Config;
use error::TuxDriveResult;
use forest::{info::BasicNodeInfo, DirectoryAddOptions, PathForest};

use crate::watcher::Watcher;

#[macro_export]
macro_rules! path {
    ($($comp:expr), *) => {
        {
            let mut new_path = std::path::PathBuf::new();
            $(new_path.push(&$comp);)*
            new_path
        }
    };
}

mod config;
mod error;
mod forest;
mod watcher;

#[cfg(not(unix))]
compile_error!("Cannot compile TuxDrive on Windows!");

fn main() {
    use std::process::exit;

    if let Err(e) = setup_and_run() {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn setup_and_run() -> TuxDriveResult<()> {
    let config = Config::read()?;
    let (mut watcher, event_recv) = Watcher::new()?;
    let mut path_forest = PathForest::<BasicNodeInfo>::new();
    for path_conf in config.paths() {
        watcher.add_directory(path_conf.path().canonicalize()?, path_conf.recursive())?;
        path_forest.add_dir_recursively(path_conf.path(), DirectoryAddOptions::new())?;
    }

    // So now we have our path_forest ready with all the paths, and our watcher has the files added
    // Therefore, path_forest can now go behind a RwLock.

    Ok(())
}
