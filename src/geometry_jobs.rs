//! A bounded executor for expensive editor operations. Rendering keeps its own worker.
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, OnceLock},
    thread::JoinHandle,
};
type Job = Box<dyn FnOnce() + Send + 'static>;
#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    shutdown: bool,
}
struct Pool {
    shared: Arc<(Mutex<Queue>, Condvar)>,
    workers: Vec<JoinHandle<()>>,
    capacity: usize,
}
impl Pool {
    fn new(workers: usize, capacity: usize) -> Self {
        let shared = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let workers = (0..workers)
            .map(|_| {
                let shared = shared.clone();
                std::thread::spawn(move || {
                    loop {
                        let job = {
                            let (lock, wake) = &*shared;
                            let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
                            while queue.jobs.is_empty() && !queue.shutdown {
                                queue = wake.wait(queue).unwrap_or_else(|e| e.into_inner());
                            }
                            if queue.shutdown {
                                return;
                            }
                            queue.jobs.pop_front().unwrap()
                        };
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                    }
                })
            })
            .collect();
        Self {
            shared,
            workers,
            capacity,
        }
    }
    fn submit(&self, job: Job) -> bool {
        let (lock, wake) = &*self.shared;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        if queue.jobs.len() >= self.capacity || queue.shutdown {
            return false;
        }
        queue.jobs.push_back(job);
        wake.notify_one();
        true
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        let (lock, wake) = &*self.shared;
        {
            let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
            queue.shutdown = true;
            queue.jobs.clear();
        }
        wake.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
pub(crate) fn spawn(job: impl FnOnce() + Send + 'static) -> bool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| Pool::new(2, 4)).submit(Box::new(job))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executor_rejects_unbounded_work_and_survives_a_failed_job() {
        let pool = Pool::new(1, 1);
        let (started, rx) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        assert!(pool.submit(Box::new(move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
        })));
        rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(pool.submit(Box::new(|| {})));
        assert!(!pool.submit(Box::new(|| {})));
        release.send(()).unwrap();
        // Shutdown joins the active worker and releases all queued snapshots.
        drop(pool);
    }
}
