use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::{TuxDriveError, TuxDriveResult};

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config(Vec<PathConfig>);

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PathConfig {
    path: PathBuf,
    recursive: bool,
}

macro_rules! path {
    ($($comp:expr), *) => {
        {
            let mut new_path = std::path::PathBuf::new();
            $(new_path.push(&$comp);)*
            new_path
        }
    };
}

impl Config {
    pub fn read() -> TuxDriveResult<Self> {
        let home_dir = dirs::home_dir().ok_or(TuxDriveError::HomeDirNotFound)?;
        let config_dir = dirs::config_dir().ok_or(TuxDriveError::ConfigDirNotFound)?;
        let config_paths = vec![
            path![home_dir, ".tuxdriver.json"],
            path![config_dir, ".tuxdriver.json"],
            path![config_dir, ".config", "tuxdirver", "tuxdriver.json"],
            path!["tuxdriver.json"],
        ];
        if let Some(config_path) = config_paths
            .into_iter()
            .find(|path| path.exists() && path.is_file())
        {
            let file = File::open(&config_path)?;
            Config::from_reader(file)
        } else {
            Err(TuxDriveError::ConfigFileNotFound)
        }
    }

    fn from_reader<R: io::Read>(rdr: R) -> TuxDriveResult<Self> {
        let config: Config = serde_json::from_reader(rdr)?;
        if let Some(path_cfg) = config
            .0
            .iter()
            .find(|path_cfg| !path_cfg.path.is_absolute())
        {
            Err(TuxDriveError::PathNotAbs(
                path_cfg.path.display().to_string(),
            ))
        } else {
            Ok(config)
        }
    }

    pub fn paths(&self) -> &[PathConfig] {
        &self.0
    }
}

impl PathConfig {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn recursive(&self) -> bool {
        self.recursive
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    pub fn test_correct() {
        let config_text = r#"
[
    {
        "path": "/home/foo/rec_dir",
        "recursive": true
    },
    {
        "path": "/home/foo/non_rec_dir",
        "recursive": false
    }
]
"#;
        let config = Config::from_reader(Cursor::new(config_text)).unwrap();
        let expected_config = Config(vec![
            PathConfig {
                path: PathBuf::from("/home/foo/rec_dir"),
                recursive: true,
            },
            PathConfig {
                path: PathBuf::from("/home/foo/non_rec_dir"),
                recursive: false,
            },
        ]);
        assert_eq!(config, expected_config);
    }

    #[test]
    pub fn test_no_path() {
        let config_text = r#"
[
    {
        "path": "/home/foo/rec_dir",
        "recursive": true
    },
    {
        "recursive": false
    }
]
"#;
        let config = Config::from_reader(Cursor::new(config_text));
        assert!(config.is_err());
    }

    #[test]
    pub fn test_no_recursive() {
        let config_text = r#"
[
    {
        "path": "/home/foo/rec_dir",
        "recursive": true
    },
    {
        "path": "/home/foo/rec_dir",
    }
]
"#;
        let config = Config::from_reader(Cursor::new(config_text));
        assert!(config.is_err());
    }

    #[test]
    pub fn test_not_abs_path() {
        let config_text = r#"
[
    {
        "path": "foo/rec_dir",
        "recursive": true
    }
]
"#;
        let config = Config::from_reader(Cursor::new(config_text));
        assert!(config.is_err());
        assert!(matches!(config, Err(TuxDriveError::PathNotAbs(_))));
    }
}
