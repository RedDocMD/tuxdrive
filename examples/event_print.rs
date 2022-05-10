use std::{env, thread};
use tuxdrive::watcher::{WatchEventKind, Watcher};

fn main() {
    let args = env::args().collect::<Vec<_>>();
    const POLL_INTERVAL: u64 = 1;
    let (mut file_watcher, event_recv) = Watcher::<{ POLL_INTERVAL }>::new().unwrap();
    file_watcher.add_directory(&args[1], true).unwrap();
    thread::spawn(move || file_watcher.start_polling());
    while let Ok(ev) = event_recv.recv() {
        println!("{},{}", ev.path.display(), event_kind_to_string(ev.kind));
    }
}

fn event_kind_to_string(kind: WatchEventKind) -> &'static str {
    match kind {
        WatchEventKind::Create => "Create",
        WatchEventKind::Delete => "Delete",
        WatchEventKind::Written => "Written",
        WatchEventKind::Chmod => "Chmod",
    }
}
