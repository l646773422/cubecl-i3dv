//! Bounded, process-lifetime CPU executor for Windows native pipeline creation.
//! NVIDIA 32.0.16.1062 retains a Timer/Semaphore pair for each exited compiler
//! thread. Reusing a thread bounds that driver state across SDK session recreation.
//! Completed jobs leave no device/module/pipeline references on the worker.
use std::sync::{OnceLock, mpsc};

type Job = Box<dyn FnOnce() + Send>;
static WORKER: OnceLock<Result<mpsc::SyncSender<Job>, String>> = OnceLock::new();
std::thread_local! {
    static ON_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn run<T: Send + 'static>(operation: impl FnOnce() -> T + Send + 'static) -> T {
    if ON_WORKER.get() {
        return operation();
    }
    let worker = WORKER
        .get_or_init(|| {
            let (send, receive) = mpsc::sync_channel::<Job>(1);
            std::thread::Builder::new()
                .name("i3dv-sdk-compiler".into())
                .spawn(move || {
                    ON_WORKER.set(true);
                    while let Ok(job) = receive.recv() {
                        job();
                    }
                })
                .map(|_detached_thread| send)
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .unwrap_or_else(|error| panic!("SDK compiler thread unavailable: {error}"));
    let (send_result, receive_result) = mpsc::sync_channel(1);
    worker
        .send(Box::new(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
            let _ = send_result.send(result);
        }))
        .unwrap_or_else(|_| panic!("SDK compiler thread stopped"));
    match receive_result
        .recv()
        .unwrap_or_else(|_| panic!("SDK compiler result disconnected"))
    {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
