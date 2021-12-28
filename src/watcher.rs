use std::path::Path;

use crate::error::{TuxDriveError, TuxDriveResult};
use crate::forest::PathForest;

pub struct Watcher {
    forest: PathForest<ModTimeInfo>,
}

impl Watcher {
    pub fn new() -> Self {
        Self {
            forest: PathForest::new(),
        }
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
            self.forest.add_dir_recursively(path)?;
        } else {
            self.forest.add_dir_non_recursively(path)?;
        }
        // Update the times
        self.update_times()
    }

    fn update_times(&mut self) -> TuxDriveResult<()> {
        self.forest.dfs_mut(|path, time_info, is_dir| {
            let old_time_info = *time_info;
            time_info.update_times(path)?;
            Ok(time_info.updated_since(&old_time_info) && is_dir)
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ModTimeInfo {
    mtime: i64,
    ctime: i64,
}

impl ModTimeInfo {
    fn update_times<P: AsRef<Path>>(&mut self, path: P) -> TuxDriveResult<()> {
        use nix::sys;

        let path = path.as_ref();
        let stat = sys::stat::stat(path)?;
        self.mtime = stat.st_mtime;
        self.ctime = stat.st_ctime;

        Ok(())
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
