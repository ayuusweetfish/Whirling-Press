use notify::{Config, Event, EventKindMask, RecursiveMode, Watcher};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

pub struct DirWatcher {
  flag: Arc<AtomicBool>,
  // `Option`-wrapped for ordered dropping
  thread: Option<JoinHandle<()>>,
  watcher: Option<notify::RecommendedWatcher>,
}

impl DirWatcher {
  pub fn new<P: AsRef<std::path::Path>>(p: P) -> Result<Self, Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.configure(Config::default().with_event_kinds(EventKindMask::CORE))?;
    watcher.watch(p.as_ref(), RecursiveMode::Recursive)?;

    let flag = Arc::new(AtomicBool::new(false));
    let flag_t = Arc::clone(&flag);

    let thread = std::thread::spawn(move || {
      for res in rx {
        match res {
          Ok(event) =>
            // Previous `watcher.configure()` may fail; filter here to ensure
            if EventKindMask::CORE.matches(&event.kind) {
              flag_t.store(true, Ordering::Release);
            }
          Err(e) => eprintln!("notify::Watcher error: {:?}", e),
        }
      }
    });

    Ok(Self {
      flag,
      thread: Some(thread),
      watcher: Some(watcher),
    })
  }

  pub fn take_changed(&self) -> bool {
    self.flag.swap(false, Ordering::AcqRel)
  }
}

impl Drop for DirWatcher {
  fn drop(&mut self) {
    // Dropping the watcher closes the thread's channel and lets it terminate
    self.watcher.take();
    if let Some(t) = self.thread.take() {
      let _ = t.join();
    }
  }
}
