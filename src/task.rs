/// Async Task System — cooperative multitasking for the kernel.
///
/// Provides a simple executor that drives `async fn` tasks to completion.
/// Each task is a pinned, heap-allocated future with a unique ID.
///
/// This is the foundation for the chat interface, I/O multiplexing,
/// and eventually the LLM request pipeline — all as async tasks.

use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

/// Unique task identifier. Monotonically increasing, never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A kernel task — a pinned, heap-allocated future with a unique ID.
pub struct Task {
    pub id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task from any future that returns `()`.
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    /// Poll this task once, returning whether it completed.
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

/// Simple cooperative executor — runs tasks round-robin until all complete.
///
/// Uses a simple FIFO queue. Tasks that return `Pending` go to the back
/// of the queue. Tasks that return `Ready` are dropped.
///
/// This is intentionally simple — no waker-based wake-up, no priority
/// scheduling. The kernel is single-threaded and cooperative. A more
/// sophisticated executor with proper wakers comes later.
pub mod simple_executor {
    use super::*;
    use alloc::collections::VecDeque;
    use core::task::{RawWaker, RawWakerVTable, Waker};

    /// A simple round-robin executor.
    pub struct SimpleExecutor {
        task_queue: VecDeque<Task>,
    }

    impl SimpleExecutor {
        /// Create a new empty executor.
        pub fn new() -> SimpleExecutor {
            SimpleExecutor {
                task_queue: VecDeque::new(),
            }
        }

        /// Spawn a task into the executor.
        pub fn spawn(&mut self, task: Task) {
            self.task_queue.push_back(task);
        }

        /// Run all tasks to completion.
        ///
        /// Polls each task once per round. Pending tasks get re-queued.
        /// Returns when all tasks have completed.
        ///
        /// **Note:** This busy-loops on pending tasks. The improved executor
        /// (Phase 3+) will use proper wakers to sleep between polls.
        pub fn run(&mut self) {
            while let Some(mut task) = self.task_queue.pop_front() {
                let waker = dummy_waker();
                let mut context = Context::from_waker(&waker);
                match task.poll(&mut context) {
                    Poll::Ready(()) => {
                        // Task completed — drop it
                        serial_println!("[TASK] Task {:?} completed", task.id);
                    }
                    Poll::Pending => {
                        // Task not done — re-queue it
                        self.task_queue.push_back(task);
                    }
                }
            }
        }
    }

    /// Create a dummy waker that does nothing when woken.
    ///
    /// This is fine for the simple executor since we poll all tasks
    /// every round anyway. The proper waker-based executor comes later.
    fn dummy_waker() -> Waker {
        unsafe { Waker::from_raw(dummy_raw_waker()) }
    }

    fn dummy_raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            dummy_raw_waker()
        }

        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(core::ptr::null(), vtable)
    }
}
