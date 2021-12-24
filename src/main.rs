use config::Config;

mod config;
mod error;
mod watcher;

fn main() {
    let config = Config::read();
}
