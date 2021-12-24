use std::{path::Path, sync::mpsc, time::Duration};

use notify::{DebouncedEvent, RecursiveMode, Watcher as _};

use crate::error::{TuxDriveError, TuxDriveResult};

pub struct Watcher<const DEBOUNCE_TIME: u64> {
    watcher: notify::RecommendedWatcher,
}

impl<const DEBOUNCE_TIME: u64> Watcher<{ DEBOUNCE_TIME }> {
    pub fn new() -> TuxDriveResult<(Self, mpsc::Receiver<DebouncedEvent>)> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::watcher(tx, Duration::from_secs(DEBOUNCE_TIME))?;
        Ok((Self { watcher }, rx))
    }

    pub fn add_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
        recursive: bool,
    ) -> TuxDriveResult<()> {
        let path = path.as_ref();
        if !path.is_dir() {
            return Err(TuxDriveError::NotDirectory(path.display().to_string()));
        }
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        self.watcher.watch(path, mode)?;
        Ok(())
    }
}
