use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use crate::error::TuxDriveResult;

pub mod info;

#[derive(Debug)]
pub struct PathTree<T> {
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

#[derive(Debug, Clone, Copy)]
pub struct DirectoryAddOptions {
    ignore_not_found: bool,
    ignore_no_access: bool,
}

impl DirectoryAddOptions {
    pub fn new() -> Self {
        Self {
            ignore_not_found: true,
            ignore_no_access: true,
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

    /// root_path: Must belong to tree
    pub fn remove_path<P: AsRef<Path>>(&mut self, root_path: P, path: P) -> bool {
        self.trees
            .get_mut(root_path.as_ref())
            .unwrap()
            .remove_path(path)
    }

    pub fn add_dir_recursively<P: AsRef<Path>>(
        &mut self,
        dir_path: P,
        options: DirectoryAddOptions,
    ) -> TuxDriveResult<()> {
        let dir_path = dir_path.as_ref();
        assert!(dir_path.is_dir());
        self.add_path(dir_path, dir_path, T::default(), true);
        match self.add_dir_rec_intern(dir_path, dir_path, options)? {
            RecursiveBehaviour::Nothing => {}
            RecursiveBehaviour::Delete => {
                self.remove_path(dir_path, dir_path);
            }
        };
        Ok(())
    }

    pub fn add_dir_non_recursively<P: AsRef<Path>>(&mut self, dir_path: P) -> TuxDriveResult<()> {
        let dir_path = dir_path.as_ref();
        assert!(dir_path.is_dir());
        let entries = match dir_path.read_dir() {
            Ok(v) => {
                self.add_path(dir_path, dir_path, T::default(), true);
                v
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::PermissionDenied {
                    return Ok(());
                } else {
                    return Err(err.into());
                }
            }
        };
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
            let is_dir = match entry.file_type() {
                Ok(v) => {
                    if !v.is_dir() && !v.is_file() {
                        continue;
                    }
                    v.is_dir()
                }
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
            let path = entry.path();
            let info = T::default();
            self.add_path(dir_path, &path, info, is_dir);
        }
        Ok(())
    }

    fn add_dir_rec_intern(
        &mut self,
        root_path: &Path,
        dir_path: &Path,
        options: DirectoryAddOptions,
    ) -> TuxDriveResult<RecursiveBehaviour> {
        let entries = match dir_path.read_dir() {
            Ok(v) => v,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::PermissionDenied {
                    return Ok(RecursiveBehaviour::Delete);
                } else {
                    return Err(err.into());
                }
            }
        };
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
            let is_dir = match entry.file_type() {
                Ok(v) => {
                    if !v.is_dir() && !v.is_file() {
                        continue;
                    }
                    v.is_dir()
                }
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
            let path = entry.path();
            let info = T::default();
            self.add_path(root_path, &path, info, is_dir);
            if is_dir {
                match self.add_dir_rec_intern(root_path, &path, options)? {
                    RecursiveBehaviour::Nothing => {}
                    RecursiveBehaviour::Delete => {
                        self.remove_path(root_path, &path);
                    }
                }
            }
        }
        Ok(RecursiveBehaviour::Nothing)
    }

    /// If func returns true, then recurse further, otherwise not.
    /// Stops at the first error.
    pub fn dfs_mut<F>(&mut self, func: F) -> TuxDriveResult<()>
    where
        F: FnMut(&Path, DfsMutInfo<T>) -> TuxDriveResult<DfsFuncBehaviour> + Copy,
    {
        let mut keys_to_delete = Vec::new();
        for (key, tree) in self.trees.iter_mut() {
            match tree.dfs_mut(func)? {
                RecursiveBehaviour::Nothing => {}
                RecursiveBehaviour::Delete => keys_to_delete.push(key.clone()),
            }
        }
        for key in keys_to_delete {
            self.trees.remove(&key);
        }
        Ok(())
    }

    pub fn trees_mut(&mut self) -> impl Iterator<Item = &mut PathTree<T>> {
        self.trees.iter_mut().map(|(_, tree)| tree)
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
        let residual_path_comps = self.strip_root(path.as_ref());
        self.node.add_node_rec(&residual_path_comps, info, is_dir);
    }

    /// Precondition:
    /// - `path` must be cannonical
    /// - `path` must be compatible with this tree
    fn remove_path<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let residual_path_comps = self.strip_root(path.as_ref());
        self.node.remove_node_rec(&residual_path_comps)
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

    fn strip_root<'a>(&self, path: &'a Path) -> Vec<Component<'a>> {
        assert!(self.is_path_compatible(&path));
        let root_path_comps_len = self.root_path().components().count();
        path.components().skip(root_path_comps_len - 1).collect()
    }

    pub fn dfs_mut<F>(&mut self, func: F) -> TuxDriveResult<RecursiveBehaviour>
    where
        F: FnMut(&Path, DfsMutInfo<T>) -> TuxDriveResult<DfsFuncBehaviour> + Copy,
        T: Default,
    {
        let mut root_path = self.root_path();
        self.node.dfs_mut(&mut root_path, func)
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

    fn remove_node_rec(&mut self, comps: &[Component<'_>]) -> bool {
        assert!(!comps.is_empty());
        if comps.len() == 1 {
            let name = comps[0].as_os_str();
            self.children.remove(name).is_some()
        } else if let Some(child) = self.children.get_mut(comps[0].as_os_str()) {
            child.remove_node_rec(&comps[1..])
        } else {
            false
        }
    }

    fn dfs_mut<F>(
        &mut self,
        curr_path: &mut PathBuf,
        mut func: F,
    ) -> TuxDriveResult<RecursiveBehaviour>
    where
        F: FnMut(&Path, DfsMutInfo<T>) -> TuxDriveResult<DfsFuncBehaviour> + Copy,
        T: Default,
    {
        fn recurse_downwards<T, F>(
            node: &mut PathNode<T>,
            curr_path: &mut PathBuf,
            func: F,
        ) -> TuxDriveResult<RecursiveBehaviour>
        where
            F: FnMut(&Path, DfsMutInfo<T>) -> TuxDriveResult<DfsFuncBehaviour> + Copy,
            T: Default,
        {
            let mut keys_to_delete = Vec::new();
            for (key, node) in node.children.iter_mut() {
                // Name can be empty only at the root
                assert!(node.name.is_some());
                curr_path.push(node.name.as_ref().unwrap());
                match node.dfs_mut(curr_path, func)? {
                    RecursiveBehaviour::Nothing => {}
                    RecursiveBehaviour::Delete => keys_to_delete.push(key.clone()),
                }
                curr_path.pop();
            }
            for key in keys_to_delete {
                node.children.remove(&key);
            }
            Ok(RecursiveBehaviour::Nothing)
        }

        fn add_new_paths<T: Default>(node: &mut PathNode<T>, new_paths: Vec<PathBuf>) {
            for path in new_paths {
                let name = path.file_name().map(OsString::from);
                let new_node = PathNode::new(name.clone(), T::default(), path.is_dir());
                node.children.insert(name.unwrap(), new_node);
            }
        }

        match func(curr_path, self.get_dfs_mut_info(curr_path))? {
            DfsFuncBehaviour::Continue => recurse_downwards(self, curr_path, func),
            DfsFuncBehaviour::Stop => Ok(RecursiveBehaviour::Nothing),
            DfsFuncBehaviour::Delete => Ok(RecursiveBehaviour::Delete),
            DfsFuncBehaviour::AddAndContinue(paths) => {
                add_new_paths(self, paths);
                recurse_downwards(self, curr_path, func)
            }
            DfsFuncBehaviour::AddAndStop(paths) => {
                add_new_paths(self, paths);
                Ok(RecursiveBehaviour::Nothing)
            }
        }
    }

    fn get_dfs_mut_info(&mut self, path: &Path) -> DfsMutInfo<T> {
        let children_paths = self
            .children
            .values()
            .map(|node| path![path, node.name.as_ref().unwrap_or(&OsString::new())])
            .collect();
        DfsMutInfo {
            children_paths,
            info: &mut self.info,
            is_dir: self.is_dir,
        }
    }
}

#[derive(Debug)]
pub enum DfsFuncBehaviour {
    Continue,
    Stop,
    Delete,
    AddAndContinue(Vec<PathBuf>),
    AddAndStop(Vec<PathBuf>),
}

#[derive(Debug)]
pub enum RecursiveBehaviour {
    Nothing,
    Delete,
}

#[derive(Debug)]
pub struct DfsMutInfo<'info, T> {
    pub children_paths: HashSet<PathBuf>,
    pub info: &'info mut T,
    pub is_dir: bool,
}
