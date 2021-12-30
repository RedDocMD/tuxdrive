use std::{io::Write, thread};

use colored::*;
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

    env_logger::builder()
        .format(|buf, rec| {
            let line = rec
                .line()
                .map_or(String::new(), |line| format!(":{}", line));
            let file = rec
                .file()
                .map_or(String::new(), |file| format!(" {}", file));
            let prelude = format!("[{}{}{}]", rec.level(), file, line);
            writeln!(buf, "{} {}", prelude.cyan(), rec.args())
        })
        .write_style(env_logger::WriteStyle::Always)
        .init();

    if let Err(e) = setup_and_run() {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

const POLL_INTERVAL_SECS: u64 = 5;

fn setup_and_run() -> TuxDriveResult<()> {
    let config = Config::read()?;
    let (mut watcher, event_recv) = Watcher::<{ POLL_INTERVAL_SECS }>::new()?;
    let mut path_forest = PathForest::<BasicNodeInfo>::new();
    for path_conf in config.paths() {
        watcher.add_directory(path_conf.path().canonicalize()?, path_conf.recursive())?;
        path_forest.add_dir_recursively(path_conf.path(), DirectoryAddOptions::new())?;
    }

    // Start the watcher
    thread::spawn(move || watcher.start_polling());

    while let Ok(event) = event_recv.recv() {
        println!("{:?}", event);
    }

    // So now we have our path_forest ready with all the paths, and our watcher has the files added
    // Therefore, path_forest can now go behind a RwLock.

    Ok(())
}
