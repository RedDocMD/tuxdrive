use std::path::Path;
use std::sync::{Arc, RwLock};

use config::Config;
use error::TuxDriveResult;
use forest::{info::NodeInfo, PathForest};

use crate::watcher::Watcher;

use self::forest::info::BasicNodeInfo;

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
    let mut watcher = Watcher::new()?;
    let mut path_forest = PathForest::new();
    for path_conf in config.paths() {
        watcher.add_directory(path_conf.path().canonicalize()?, path_conf.recursive())?;
        add_dir_recursively(path_conf.path(), &mut path_forest)?;
    }

    // So now we have our path_forest ready with all the paths, and our watcher has the files added
    // Therefore, path_forest can now go behind a RwLock.
    let path_forest = Arc::new(RwLock::new(path_forest));

    Ok(())
}

fn add_dir_recursively(
    dir_path: &Path,
    forest: &mut PathForest<BasicNodeInfo>,
) -> TuxDriveResult<()> {
    fn add_dir_intern(
        root_path: &Path,
        dir_path: &Path,
        forest: &mut PathForest<BasicNodeInfo>,
    ) -> TuxDriveResult<()> {
        for entry in dir_path.read_dir()? {
            let entry = entry?;
            let is_dir = entry.file_type()?.is_dir();
            let path = entry.path();
            let info = BasicNodeInfo::default().with_is_dir(is_dir);
            forest.add_path(root_path, &path, info);
            if is_dir {
                add_dir_intern(root_path, &path, forest)?;
            }
        }
        Ok(())
    }

    add_dir_intern(dir_path, dir_path, forest)
}
