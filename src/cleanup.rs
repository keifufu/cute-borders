use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[derive(Debug, Default)]
pub struct Cleanup {
  hooks: Vec<thread::JoinHandle<()>>,
  run: Arc<Mutex<bool>>,
  go: Arc<Condvar>,
}

impl Cleanup {
  pub fn add(&mut self, f: impl FnOnce() + Send + 'static) {
    let run = self.run.clone();
    let go = self.go.clone();

    let t = thread::spawn(move || {
      let mut run = run.lock().unwrap();

      while !*run {
        run = go.wait(run).unwrap();
      }

      f();
    });
    self.hooks.push(t);
  }
}

impl Drop for Cleanup {
  fn drop(&mut self) {
    *self.run.lock().unwrap() = true;
    self.go.notify_all();

    for h in self.hooks.drain(..) {
      h.join().unwrap();
    }
  }
}
