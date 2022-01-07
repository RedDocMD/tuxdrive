#[macro_export]
macro_rules! path {
    ($($comp:expr), *) => {
        {
            let mut new_path = std::path::PathBuf::new();
            $(new_path.push(&$comp);)*
            new_path
        }
    };
}

pub mod atomic;
pub mod config;
pub mod error;
pub mod forest;
pub mod reader;
pub mod watcher;

#[cfg(not(unix))]
compile_error!("Cannot compile TuxDrive on Non-Unix environments!");
