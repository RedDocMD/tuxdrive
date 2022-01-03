use std::path::PathBuf;

use crossbeam::channel::{Receiver, Sender};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::error::TuxDriveResult;

#[derive(Debug)]
pub struct ReadCommand {
    path: PathBuf,
    kind: ReadCommandKind,
}

#[derive(Debug)]
pub enum ReadCommandKind {
    Data,
    Permission,
}

pub enum ReadData {
    Data(Vec<u8>),
    Permission(FilePermission),
    Delete,
}

#[derive(Debug)]
pub struct NormalPermission {
    pub read: bool,
    pub write: bool,
    pub exectute: bool,
}

#[derive(Debug)]
pub struct SpecialPermission {
    pub suid: bool,
    pub sgid: bool,
    pub sticky: bool,
}

#[derive(Debug)]
pub struct FilePermission {
    pub user: NormalPermission,
    pub group: NormalPermission,
    pub other: NormalPermission,
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
