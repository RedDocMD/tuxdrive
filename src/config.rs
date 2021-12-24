use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config(Vec<PathConfig>);

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct PathConfig {
    path: PathBuf,
    recursive: bool,
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
