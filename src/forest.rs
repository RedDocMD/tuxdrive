use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
struct PathTree {
    parent_path: Option<PathBuf>,
    node: PathNode,
}

#[derive(Debug)]
struct PathNode {
    name: Option<OsString>,
    children: HashMap<OsString, PathNode>,
    info: NodeInfo,
}

#[derive(Debug)]
pub struct PathForest {
    /// Map from root path to corresponding tree
    trees: HashMap<PathBuf, PathTree>,
}

#[derive(Debug, Default)]
pub struct NodeInfo {
    is_dir: bool,
}

impl NodeInfo {
    pub fn with_is_dir(mut self, is_dir: bool) -> Self {
        self.is_dir = is_dir;
        self
    }
}

impl PathForest {
    pub fn new() -> Self {
        Self {
            trees: HashMap::new(),
        }
    }

    pub fn add_path<P: AsRef<Path>>(&mut self, root_path: &Path, path: P, info: NodeInfo) {
        if let Some(tree) = self.trees.get_mut(root_path) {
            tree.add_path(path, info);
        } else {
            let mut new_tree = PathTree::new(root_path);
            new_tree.add_path(path, info);
            self.trees.insert(PathBuf::from(root_path), new_tree);
        }
    }
}

impl PathTree {
    /// Precondition: `root_path` must be cannonical
    fn new<P: AsRef<Path>>(root_path: P) -> Self {
        let root_path = PathBuf::from(root_path.as_ref());
        assert!(root_path.exists() && root_path.is_dir());
        let parent_path = root_path.parent().map(PathBuf::from);
        let root_name = root_path.file_name().map(OsString::from);
        let info = NodeInfo::default().with_is_dir(true);
        let node = PathNode::new(root_name, info);
        Self { parent_path, node }
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

    /// Precondition:
    /// - `path` must be cannonical
    /// - `path` must be compatible with this tree
    fn add_path<P: AsRef<Path>>(&mut self, path: P, info: NodeInfo) {
        assert!(self.is_path_compatible(&path));
        let root_path_comps_len = self.root_path().components().count();
        let residual_path_comps: Vec<_> = path
            .as_ref()
            .components()
            .skip(root_path_comps_len)
            .collect();
        self.node.add_node_rec(&residual_path_comps, info);
    }
}

impl PathNode {
    fn new(name: Option<OsString>, info: NodeInfo) -> Self {
        Self {
            name,
            info,
            children: HashMap::new(),
        }
    }

    fn add_node_rec(&mut self, comps: &[Component<'_>], info: NodeInfo) {
        assert!(!comps.is_empty());
        if comps.len() == 1 {
            let name = comps[0].as_os_str().to_os_string();
            let new_node = PathNode::new(Some(name.clone()), info);
            self.children.insert(name, new_node);
        } else if let Some(child) = self.children.get_mut(comps[0].as_os_str()) {
            child.add_node_rec(&comps[1..], info);
        } else {
            let name = comps[0].as_os_str().to_os_string();
            let mut new_node =
                PathNode::new(Some(name.clone()), NodeInfo::default().with_is_dir(true));
            new_node.add_node_rec(&comps[1..], info);
            self.children.insert(name, new_node);
        }
    }
}
