use std::{fs::File, path::PathBuf};

use serde::Deserialize;

use crate::error::{TuxDriveError, TuxDriveResult};

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config(Vec<PathConfig>);

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct PathConfig {
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
        for config_path in &config_paths {
            if config_path.exists() && config_path.is_file() {
                let file = File::open(&config_path)?;
                let config: Config = serde_json::from_reader(&file)?;
                return Ok(config);
            }
        }
        Err(TuxDriveError::ConfigFileNotFound)
    }
}

#[cfg(test)]
mod test {
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
        let config: Config = serde_json::from_str(config_text).unwrap();
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
        let config: Result<Config, _> = serde_json::from_str(config_text);
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
        let config: Result<Config, _> = serde_json::from_str(config_text);
        assert!(config.is_err());
    }
}
