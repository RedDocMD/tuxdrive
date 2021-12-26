#[derive(Debug, Default)]
pub struct BasicNodeInfo {
    is_dir: bool,
}

impl NodeInfo for BasicNodeInfo {
    fn with_is_dir(mut self, is_dir: bool) -> Self {
        self.is_dir = is_dir;
        self
    }
}

pub trait NodeInfo: Default {
    fn with_is_dir(self, is_dir: bool) -> Self;
}
