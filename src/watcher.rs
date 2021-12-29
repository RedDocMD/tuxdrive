use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crossbeam::channel::{Receiver, Sender};
use crossbeam::sync::WaitGroup;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::error::{TuxDriveError, TuxDriveResult};
use crate::forest::{DfsFuncBehaviour, DirectoryAddOptions, PathForest, PathTree};

pub struct Watcher<const POLL_INTERVAL_SECS: u64> {
    forest: PathForest<ModTimeInfo>,
    sender: Sender<WatchEvent>,
    pool: ThreadPool,
}

const MAX_NUM_THREADS: usize = 4;

impl<const POLL_INTERVAL_SECS: u64> Watcher<{ POLL_INTERVAL_SECS }> {
    pub fn new() -> TuxDriveResult<(Self, Receiver<WatchEvent>)> {
        let (tx, rx) = crossbeam::channel::unbounded();
        let num_threads = usize::max(num_cpus::get(), MAX_NUM_THREADS);
        let pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;
        let watcher = Self {
            forest: PathForest::new(),
            sender: tx,
            pool,
        };
        Ok((watcher, rx))
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
        if recursive {
            self.forest
                .add_dir_recursively(path, DirectoryAddOptions::new())?;
        } else {
            self.forest.add_dir_non_recursively(path)?;
        }
        // Update the times
        self.update_times()
    }

    fn update_times(&mut self) -> TuxDriveResult<()> {
        self.forest.dfs_mut(|path, dfs_info| {
            if !path.exists() {
                return Ok(DfsFuncBehaviour::Delete);
            }
            let old_time_info = *dfs_info.info;
            match dfs_info.info.update_times(path)? {
                PathAction::Delete => return Ok(DfsFuncBehaviour::Delete),
                PathAction::Nothing => {}
            }
            let action = if dfs_info.info.updated_since(&old_time_info) && dfs_info.is_dir {
                DfsFuncBehaviour::Continue
            } else {
                DfsFuncBehaviour::Stop
            };
            Ok(action)
        })
    }

    fn poll(&mut self) -> TuxDriveResult<()> {
        let wg = WaitGroup::new();
        for tree in self.forest.trees_mut() {
            let wg = wg.clone();
            self.pool.install(|| {
                let res = poll_tree(&self.sender, tree);
                drop(wg);
                res
            })?;
        }
        wg.wait();
        Ok(())
    }

    /// Starts the polling of the Watcher.
    /// Polls once every POLL_INTERVAL_SECS (approximately).
    /// Probably never returns, execpt on errors.
    /// You probably should run this function on a separate thread.
    pub fn start_polling(&mut self) -> TuxDriveResult<()> {
        loop {
            self.poll()?;
            thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        }
    }
}

fn poll_tree(sender: &Sender<WatchEvent>, tree: &mut PathTree<ModTimeInfo>) -> TuxDriveResult<()> {
    tree.dfs_mut(|path, dfs_info| {
        if !path.exists() {
            sender
                .send(WatchEvent::new(path, WatchEventKind::Delete))
                .unwrap();
            return Ok(DfsFuncBehaviour::Delete);
        }
        let old_time_info = *dfs_info.info;
        match dfs_info.info.update_times(path)? {
            PathAction::Nothing => {}
            PathAction::Delete => {
                sender
                    .send(WatchEvent::new(path, WatchEventKind::Delete))
                    .unwrap();
                return Ok(DfsFuncBehaviour::Delete);
            }
        }
        if dfs_info.is_dir {
            // Handle newly created directories/files
            let entries = match path.read_dir() {
                Ok(v) => v,
                Err(err) => {
                    if err.kind() == ErrorKind::NotFound
                        || err.kind() == ErrorKind::PermissionDenied
                    {
                        return Ok(DfsFuncBehaviour::Delete);
                    } else {
                        return Err(err.into());
                    }
                }
            };
            let mut new_paths = Vec::new();
            for entry in entries {
                let entry = match entry {
                    Ok(v) => v,
                    Err(err) => {
                        if err.kind() == ErrorKind::NotFound
                            || err.kind() == ErrorKind::PermissionDenied
                        {
                            continue;
                        } else {
                            return Err(err.into());
                        }
                    }
                };
                if !dfs_info.children_paths.contains(&entry.path()) {
                    // Newly found path
                    new_paths.push(entry.path());
                    sender
                        .send(WatchEvent::new(entry.path(), WatchEventKind::Create))
                        .unwrap();
                }
            }

            // Handle recursion
            if dfs_info.info.updated_since(&old_time_info) {
                if new_paths.is_empty() {
                    Ok(DfsFuncBehaviour::AddAndContinue(new_paths))
                } else {
                    Ok(DfsFuncBehaviour::Continue)
                }
            } else if new_paths.is_empty() {
                Ok(DfsFuncBehaviour::AddAndStop(new_paths))
            } else {
                Ok(DfsFuncBehaviour::Stop)
            }
        } else {
            if dfs_info.info.modified_since(&old_time_info) {
                sender
                    .send(WatchEvent::new(path, WatchEventKind::Written))
                    .unwrap();
            } else if dfs_info.info.modified_since(&old_time_info) {
                sender
                    .send(WatchEvent::new(path, WatchEventKind::Chmod))
                    .unwrap();
            }
            Ok(DfsFuncBehaviour::Stop)
        }
    })?;
    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
struct ModTimeInfo {
    mtime: i64,
    ctime: i64,
}

impl ModTimeInfo {
    fn update_times<P: AsRef<Path>>(&mut self, path: P) -> TuxDriveResult<PathAction> {
        use nix::sys;

        let path = path.as_ref();
        let stat = match sys::stat::stat(path) {
            Ok(stat) => stat,
            Err(err) => {
                if err == nix::Error::ENOENT || err == nix::Error::EACCES {
                    return Ok(PathAction::Delete);
                } else {
                    return Err(err.into());
                }
            }
        };
        self.mtime = stat.st_mtime;
        self.ctime = stat.st_ctime;

        Ok(PathAction::Nothing)
    }

    fn modified_since(&self, since: &Self) -> bool {
        self.mtime > since.mtime
    }

    fn changed_since(&self, since: &Self) -> bool {
        self.ctime > since.ctime
    }

    fn updated_since(&self, since: &Self) -> bool {
        self.modified_since(since) || self.changed_since(since)
    }
}

#[derive(Debug)]
pub struct WatchEvent {
    pub path: PathBuf,
    pub kind: WatchEventKind,
}

#[derive(Debug)]
pub enum WatchEventKind {
    Create,
    Delete,
    Written,
    Chmod,
}

impl WatchEvent {
    fn new<P: AsRef<Path>>(path: P, kind: WatchEventKind) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            kind,
        }
    }
}

enum PathAction {
    Nothing,
    Delete,
}
