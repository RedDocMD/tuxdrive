use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crossbeam::channel::{Receiver, Sender};
use nix::sys::stat::FileStat;
use rayon::{ThreadPool, ThreadPoolBuilder};

#[cfg(test)]
use derive_builder::Builder;

use crate::error::TuxDriveResult;

#[derive(Debug)]
pub struct ReadCommand {
    path: PathBuf,
    kind: ReadCommandKind,
    event_id: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ReadCommandKind {
    Data,
    Permission,
}

impl ReadCommand {
    fn process(&self) -> TuxDriveResult<ReadData> {
        let content = match self.kind {
            ReadCommandKind::Data => {
                if let Some(data) = read_deletable_file(&self.path)? {
                    ReadDataContent::Data(data)
                } else {
                    ReadDataContent::Delete
                }
            }
            ReadCommandKind::Permission => {
                if let Some(stat) = stat_deletable_file(&self.path)? {
                    let perm_bits = (stat.st_mode & 0o7777) as u16;
                    ReadDataContent::Permission(perm_bits.into())
                } else {
                    ReadDataContent::Delete
                }
            }
        };
        Ok(ReadData {
            content,
            event_id: self.event_id,
        })
    }
}

fn read_deletable_file<P: AsRef<Path>>(path: P) -> TuxDriveResult<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) => {
            if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::PermissionDenied {
                Ok(None)
            } else {
                Err(err.into())
            }
        }
    }
}

fn stat_deletable_file<P: AsRef<Path>>(path: P) -> TuxDriveResult<Option<FileStat>> {
    use nix::errno::Errno;
    use nix::sys::stat;

    match stat::stat(path.as_ref()) {
        Ok(stat) => Ok(Some(stat)),
        Err(err) => {
            if err == Errno::ENOENT || err == Errno::EACCES {
                Ok(None)
            } else {
                Err(err.into())
            }
        }
    }
}

#[derive(Debug)]
pub enum ReadDataContent {
    Data(Vec<u8>),
    Permission(FilePermission),
    Delete,
}

#[derive(Debug)]
pub struct ReadData {
    pub content: ReadDataContent,
    pub event_id: u32,
}

#[derive(Debug, Default, PartialEq, Eq)]
#[cfg_attr(test, derive(Builder))]
#[cfg_attr(test, builder(setter(into), default))]
pub struct NormalPermission {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl From<u8> for NormalPermission {
    fn from(perm: u8) -> Self {
        assert!(
            (perm & 0xF8) == 0,
            "Expected top 5 bits of perm byte to be 0"
        );
        Self {
            read: (0o4 & perm) != 0,
            write: (0o2 & perm) != 0,
            execute: (0o1 & perm) != 0,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
#[cfg_attr(test, derive(Builder))]
#[cfg_attr(test, builder(setter(into), default))]
pub struct SpecialPermission {
    pub suid: bool,
    pub sgid: bool,
    pub sticky: bool,
}

impl From<u8> for SpecialPermission {
    fn from(perm: u8) -> Self {
        assert!(
            (perm & 0xF8) == 0,
            "Expected top 5 bits of perm byte to be 0"
        );
        Self {
            suid: (0o4 & perm) != 0,
            sgid: (0o2 & perm) != 0,
            sticky: (0o1 & perm) != 0,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FilePermission {
    pub user: NormalPermission,
    pub group: NormalPermission,
    pub other: NormalPermission,
    pub spec: SpecialPermission,
}

impl From<u16> for FilePermission {
    fn from(perm: u16) -> Self {
        assert!(
            (perm & 0xF000) == 0,
            "Expected top 4 bits of file perm word to be 0"
        );
        let spec_bits = ((perm & 0o7000) >> 9) as u8;
        let user_bits = ((perm & 0o700) >> 6) as u8;
        let group_bits = ((perm & 0o70) >> 3) as u8;
        let other_bits = (perm & 0o7) as u8;
        Self {
            user: user_bits.into(),
            group: group_bits.into(),
            other: other_bits.into(),
            spec: spec_bits.into(),
        }
    }
}

#[derive(Debug)]
pub struct FileReader {
    command_recv: Receiver<ReadCommand>,
    data_send: Sender<ReadData>,
    pool: ThreadPool,
}

const MAX_NUM_THREADS: usize = 4;

impl FileReader {
    pub fn new() -> TuxDriveResult<(Self, Sender<ReadCommand>, Receiver<ReadData>)> {
        let (command_send, command_recv) = crossbeam::channel::unbounded();
        let (data_send, data_recv) = crossbeam::channel::unbounded();
        let num_threads = usize::max(num_cpus::get(), MAX_NUM_THREADS);
        let pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;
        let ob = Self {
            command_recv,
            data_send,
            pool,
        };
        Ok((ob, command_send, data_recv))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn regular_file_permissions() {
        // Regular file: 644
        let perm_bits = 0o0644;
        let expected_perm = FilePermission {
            user: NormalPermissionBuilder::default()
                .read(true)
                .write(true)
                .build()
                .unwrap(),
            group: NormalPermissionBuilder::default()
                .read(true)
                .build()
                .unwrap(),
            other: NormalPermissionBuilder::default()
                .read(true)
                .build()
                .unwrap(),
            spec: SpecialPermissionBuilder::default().build().unwrap(),
        };
        let perm: FilePermission = perm_bits.into();
        assert_eq!(perm, expected_perm);
    }

    #[test]
    pub fn ssh_key_file_permissions() {
        // SSH key file: 400
        let perm_bits = 0o0400;
        let expected_perm = FilePermission {
            user: NormalPermissionBuilder::default()
                .read(true)
                .build()
                .unwrap(),
            group: NormalPermissionBuilder::default().build().unwrap(),
            other: NormalPermissionBuilder::default().build().unwrap(),
            spec: SpecialPermissionBuilder::default().build().unwrap(),
        };
        let perm: FilePermission = perm_bits.into();
        assert_eq!(perm, expected_perm);
    }

    #[test]
    pub fn executable_file_permissions() {
        // Executable file: 755
        let perm_bits = 0o0755;
        let expected_perm = FilePermission {
            user: NormalPermissionBuilder::default()
                .read(true)
                .write(true)
                .execute(true)
                .build()
                .unwrap(),
            group: NormalPermissionBuilder::default()
                .read(true)
                .execute(true)
                .build()
                .unwrap(),
            other: NormalPermissionBuilder::default()
                .read(true)
                .execute(true)
                .build()
                .unwrap(),
            spec: SpecialPermissionBuilder::default().build().unwrap(),
        };
        let perm: FilePermission = perm_bits.into();
        assert_eq!(perm, expected_perm);
    }

    #[test]
    pub fn suid_executable_file_permissions() {
        // SUID Executable file: 4755
        let perm_bits = 0o4755;
        let expected_perm = FilePermission {
            user: NormalPermissionBuilder::default()
                .read(true)
                .write(true)
                .execute(true)
                .build()
                .unwrap(),
            group: NormalPermissionBuilder::default()
                .read(true)
                .execute(true)
                .build()
                .unwrap(),
            other: NormalPermissionBuilder::default()
                .read(true)
                .execute(true)
                .build()
                .unwrap(),
            spec: SpecialPermissionBuilder::default()
                .suid(true)
                .build()
                .unwrap(),
        };
        let perm: FilePermission = perm_bits.into();
        assert_eq!(perm, expected_perm);
    }
}
