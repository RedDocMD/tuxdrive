use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use crate::error::TuxDriveResult;

pub mod info;

#[derive(Debug)]
struct PathTree<T> {
    parent_path: Option<PathBuf>,
    node: PathNode<T>,
}

#[derive(Debug)]
struct PathNode<T> {
    name: Option<OsString>,
    children: HashMap<OsString, PathNode<T>>,
    info: T,
    is_dir: bool,
}

#[derive(Debug)]
pub struct PathForest<T> {
    /// Map from root path to corresponding tree
    trees: HashMap<PathBuf, PathTree<T>>,
}

impl<T> PathForest<T> {
    pub fn new() -> Self {
        Self {
            trees: HashMap::new(),
        }
    }
}

impl<T> PathForest<T>
where
    T: Default,
{
    pub fn add_path<P: AsRef<Path>>(&mut self, root_path: P, path: P, info: T, is_dir: bool) {
        let root_path = root_path.as_ref();
        if let Some(tree) = self.trees.get_mut(root_path) {
            tree.add_path(path, info, is_dir);
        } else {
            let mut new_tree = PathTree::new(root_path);
            new_tree.add_path(path, info, is_dir);
            self.trees.insert(PathBuf::from(root_path), new_tree);
        }
    }

    pub fn add_dir_recursively<P: AsRef<Path>>(&mut self, dir_path: P) -> TuxDriveResult<()> {
        let dir_path = dir_path.as_ref();
        self.add_dir_intern(dir_path, dir_path)
    }

    fn add_dir_intern(&mut self, root_path: &Path, dir_path: &Path) -> TuxDriveResult<()> {
        for entry in dir_path.read_dir()? {
            let entry = entry?;
            let is_dir = entry.file_type()?.is_dir();
            let path = entry.path();
            let info = T::default();
            self.add_path(root_path, &path, info, is_dir);
            if is_dir {
                self.add_dir_intern(root_path, &path)?;
            }
        }
        Ok(())
    }
}

impl<T> PathTree<T> {
    /// Precondition: `root_path` must be cannonical
    fn new<P: AsRef<Path>>(root_path: P) -> Self
    where
        T: Default,
    {
        let root_path = PathBuf::from(root_path.as_ref());
        assert!(root_path.exists() && root_path.is_dir());
        let parent_path = root_path.parent().map(PathBuf::from);
        let root_name = root_path.file_name().map(OsString::from);
        let info = T::default();
        let node = PathNode::new(root_name, info, true);
        Self { parent_path, node }
    }

    /// Precondition:
    /// - `path` must be cannonical
    /// - `path` must be compatible with this tree
    fn add_path<P: AsRef<Path>>(&mut self, path: P, info: T, is_dir: bool)
    where
        T: Default,
    {
        assert!(self.is_path_compatible(&path));
        let root_path_comps_len = self.root_path().components().count();
        let residual_path_comps: Vec<_> = path
            .as_ref()
            .components()
            .skip(root_path_comps_len)
            .collect();
        self.node.add_node_rec(&residual_path_comps, info, is_dir);
    }

    /// Precondition: `path` must be cannonical
    fn is_path_compatible<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().starts_with(self.root_path())
    }

    fn root_path(&self) -> PathBuf {
        assert!(self.parent_path.is_none() == self.node.name.is_none());
        if let Some(parent_path) = &self.parent_path {
            let mut root_path = parent_path.clone();
            root_path.push(self.node.name.as_ref().unwrap());
            root_path
        } else {
            PathBuf::from("/")
        }
    }
}

impl<T> PathNode<T> {
    fn new(name: Option<OsString>, info: T, is_dir: bool) -> Self {
        Self {
            name,
            info,
            children: HashMap::new(),
            is_dir,
        }
    }

    fn add_node_rec(&mut self, comps: &[Component<'_>], info: T, is_dir: bool)
    where
        T: Default,
    {
        assert!(!comps.is_empty());
        if comps.len() == 1 {
            let name = comps[0].as_os_str().to_os_string();
            let new_node = PathNode::new(Some(name.clone()), info, is_dir);
            self.children.insert(name, new_node);
        } else if let Some(child) = self.children.get_mut(comps[0].as_os_str()) {
            child.add_node_rec(&comps[1..], info, is_dir);
        } else {
            let name = comps[0].as_os_str().to_os_string();
            let mut new_node = PathNode::new(Some(name.clone()), T::default(), true);
            new_node.add_node_rec(&comps[1..], info, is_dir);
            self.children.insert(name, new_node);
        }
    }
}
