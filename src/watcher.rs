use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crossbeam::channel::{Receiver, Sender};
use crossbeam::sync::WaitGroup;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::atomic::AtomicIdGenerator;
use crate::error::{TuxDriveError, TuxDriveResult};
use crate::forest::{DfsFuncBehaviour, DfsMutInfo, DirectoryAddOptions, PathForest, PathTree};

pub struct Watcher<const POLL_INTERVAL_SECS: u64> {
    forest: PathForest<ModTimeInfo>,
    sender: Sender<WatchEvent>,
    pool: ThreadPool,
    id_gen: AtomicIdGenerator,
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
            id_gen: AtomicIdGenerator::new(),
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
                let res = poll_tree(
                    tree,
                    SendInfo {
                        sender: &self.sender,
                        id_gen: &self.id_gen,
                    },
                );
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
            log::debug!("Polling ...");
            self.poll()?;
            thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        }
    }
}

struct SendInfo<'a> {
    sender: &'a Sender<WatchEvent>,
    id_gen: &'a AtomicIdGenerator,
}

impl SendInfo<'_> {
    fn send_event<P: AsRef<Path>>(&self, path: P, kind: WatchEventKind) {
        self.sender
            .send(WatchEvent::new(path, kind, self.id_gen.next_id()))
            .unwrap();
    }
}

fn poll_tree(tree: &mut PathTree<ModTimeInfo>, send_info: SendInfo<'_>) -> TuxDriveResult<()> {
    tree.dfs_mut(|path, dfs_info| {
        log::debug!(
            "Path: {}, Is-Dir: {}, Existing children: {}",
            path.display(),
            dfs_info.is_dir,
            dfs_info.children_paths.len(),
        );

        if !path.exists() {
            send_info.send_event(path, WatchEventKind::Delete);
            return Ok(DfsFuncBehaviour::Delete);
        }

        if !path.is_dir() && !path.is_file() {
            // It is neither a file nor a directory.
            // So get rid of it.
            send_info.send_event(path, WatchEventKind::Delete);
            return Ok(DfsFuncBehaviour::Delete);
        }

        if path.is_dir() != dfs_info.is_dir {
            send_info.send_event(path, WatchEventKind::Delete);
            // We defer the "creation" until the next poll cycle
            return Ok(DfsFuncBehaviour::Delete);
        }

        let old_time_info = *dfs_info.info;
        match dfs_info.info.update_times(path)? {
            PathAction::Nothing => {}
            PathAction::Delete => {
                send_info.send_event(path, WatchEventKind::Delete);
                return Ok(DfsFuncBehaviour::Delete);
            }
        }
        log::debug!(
            "Old time: {:?}, New time: {:?}",
            old_time_info,
            dfs_info.info
        );

        if dfs_info.is_dir {
            handle_dir(path, &dfs_info, &send_info)
        } else {
            handle_file(path, &dfs_info, &old_time_info, &send_info)
        }
    })?;
    Ok(())
}

fn handle_file(
    path: &Path,
    dfs_info: &DfsMutInfo<ModTimeInfo>,
    old_time_info: &ModTimeInfo,
    send_info: &SendInfo<'_>,
) -> TuxDriveResult<DfsFuncBehaviour> {
    if dfs_info.info.modified_since(old_time_info) {
        send_info.send_event(path, WatchEventKind::Written);
    } else if dfs_info.info.changed_since(old_time_info) {
        send_info.send_event(path, WatchEventKind::Chmod);
    }
    Ok(DfsFuncBehaviour::Stop)
}

fn handle_dir(
    path: &Path,
    dfs_info: &DfsMutInfo<ModTimeInfo>,
    send_info: &SendInfo<'_>,
) -> TuxDriveResult<DfsFuncBehaviour> {
    // Handle newly created directories/files
    let entries = match path.read_dir() {
        Ok(v) => v,
        Err(err) => {
            if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::PermissionDenied {
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
                if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::PermissionDenied {
                    continue;
                } else {
                    return Err(err.into());
                }
            }
        };
        if !dfs_info.children_paths.contains(&entry.path()) {
            // Only add files and directories
            if !entry.path().is_dir() && !entry.path().is_file() {
                continue;
            }
            // Newly found path
            new_paths.push(entry.path());
            send_info.send_event(entry.path(), WatchEventKind::Create);
        }
    }

    // Handle recursion
    if !new_paths.is_empty() {
        Ok(DfsFuncBehaviour::AddAndContinue(new_paths))
    } else {
        Ok(DfsFuncBehaviour::Continue)
    }
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
    pub id: u32,
}

#[derive(Debug)]
pub enum WatchEventKind {
    // Emitted for both directories and files
    Create,

    // If directory is deleted, only emitted for it (not descendants)
    Delete,

    // Emitted only for file
    Written,

    // Emiited only for file
    Chmod,
}

impl WatchEvent {
    fn new<P: AsRef<Path>>(path: P, kind: WatchEventKind, id: u32) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            kind,
            id,
        }
    }
}

enum PathAction {
    Nothing,
    Delete,
}
