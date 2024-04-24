//! `long-running-task` is a simple library to handle and manage [long-running tasks](https://restfulapi.net/rest-api-design-for-long-running-tasks/).
//! If you want to use this crate in combination with web-frameworks you probably want to enable the feature `serde`.

#![warn(clippy::all, clippy::pedantic)]
#![warn(missing_docs)]

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Structs implementing this trait hold the current progress of a task.
pub trait Progressible {
    /// Report progress on a task, for example by increasing a progress field by 1.
    fn progress(&mut self);
}

#[derive(PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// Representation of a task's state.
pub enum TaskState<V, P>
where
    P: Progressible,
{
    /// The task is not finished yet and holds its current progress.
    Pending(P),
    /// The task is done with a value of type V.
    Done(V),
}

/// A pool to manage long-running tasks.
pub struct TaskPool<V, P>
where
    P: Progressible,
{
    pending: HashMap<Uuid, P>,
    completed: HashMap<Uuid, (Instant, V)>,
    lifespan: Option<Duration>,
}

impl<V, P> Default for TaskPool<V, P>
where
    P: Progressible,
{
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
            completed: HashMap::new(),
            lifespan: None,
        }
    }
}

/// A unique handle to a single task.
/// Does not implement clone or expose its inner fields
/// because it must be a unique reference to the task.
pub struct Handle {
    uuid: Uuid,
}

impl<V, P> TaskPool<V, P>
where
    P: Progressible + Clone,
{
    /// Configure the lifespan of tasks.
    /// `None` means that tasks will never expire.
    /// Expired tasks are purged as soon as `complete` is invoked.
    /// Specifying a lifespan is useful because clients might not always retrieve completed tasks.
    /// Without configuring a lifespan, such abandoned tasks accumulate over time and fill up memory.
    #[cfg(feature = "lifespan")]
    #[must_use]
    pub fn with_lifespan(mut self, lifespan: Option<Duration>) -> Self {
        self.lifespan = lifespan;
        self
    }

    /// Insert a task's initial progress to receive a `Handle` and `Uuid`
    /// referencing the task.
    #[must_use]
    pub fn insert(&mut self, pending: P) -> (Handle, Uuid) {
        let uuid = Uuid::new_v4();
        self.pending.insert(uuid, pending);
        (Handle { uuid }, uuid)
    }

    /// Get the task state and remove it from the pool if it is done.
    pub fn retrieve(&mut self, uuid: &Uuid) -> Option<TaskState<V, P>> {
        use TaskState::{Done, Pending};

        if let Some(p) = self.pending.get(uuid) {
            return Some(Pending(p.clone()));
        }

        self.completed.remove(uuid).map(|f| Done(f.1))
    }

    /// Report progress on a pending task.
    /// Calls `Progressible::progress` on the corresponding progress state.
    pub fn progress(&mut self, handle: &Handle) {
        match self.pending.get_mut(&handle.uuid) {
            Some(p) => p.progress(),
            None => unreachable!("Pending task not found. This should never happen because a task's handle cannot outlive the task."),
        }
    }

    /// Mark the task associated to the handle as completed with a value of type T.
    /// The handle must be passed by value so that this is the final action.
    /// As a side effect expired tasks are purged.
    #[allow(clippy::needless_pass_by_value)]
    pub fn complete(&mut self, handle: Handle, value: V) {
        self.pending.remove(&handle.uuid);
        self.purge_expired_tasks();
        self.completed.insert(handle.uuid, (Instant::now(), value));
    }

    fn purge_expired_tasks(&mut self) {
        if let Some(lifespan) = self.lifespan {
            let now = Instant::now();
            self.completed
                .retain(|_, (inserted_at, _)| now.duration_since(*inserted_at) < lifespan);
        }
    }
}

#[cfg(test)]
mod tests {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Progress {
        pub progress: usize,
        pub total: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct EmptyProgress {}

    impl Progressible for EmptyProgress {
        fn progress(&mut self) {}
    }

    impl Progressible for Progress {
        fn progress(&mut self) {
            self.progress = (self.progress + 1).min(self.total);
        }
    }

    use std::{thread, time::Duration};

    use super::Progressible;
    use crate::{TaskPool, TaskState::*};

    #[test]
    fn insert_and_get() {
        let mut pool = TaskPool::<u8, Progress>::default();
        let initial_value = Progress {
            progress: 0,
            total: 7,
        };

        let (handle, uuid) = pool.insert(initial_value);
        assert_eq!(
            pool.retrieve(&uuid),
            Some(Pending(Progress {
                progress: 0,
                total: 7
            }))
        );

        pool.progress(&handle);
        assert_eq!(
            pool.retrieve(&uuid),
            Some(Pending(Progress {
                progress: 1,
                total: 7
            }))
        );

        pool.complete(handle, 42);

        assert_eq!(get_inner_size(&pool), 1);
        assert_eq!(pool.retrieve(&uuid), Some(Done(42)));
        assert_eq!(get_inner_size(&pool), 0);
        assert_eq!(pool.retrieve(&uuid), None);
    }

    #[test]
    #[cfg(feature = "lifespan")]
    fn exceed_lifespan() {
        let lifespan = Duration::from_millis(10);
        let mut pool = TaskPool::<(), EmptyProgress>::default().with_lifespan(Some(lifespan));

        let id = insert_and_complete(&mut pool);
        thread::sleep(lifespan); // exceed time
        insert_and_complete(&mut pool); // trigger purge by completing new task

        assert_eq!(pool.retrieve(&id), None);
    }

    #[test]
    #[cfg(feature = "lifespan")]
    fn within_lifespan() {
        let lifespan = Duration::from_millis(10);
        let mut pool = TaskPool::<(), EmptyProgress>::default().with_lifespan(Some(lifespan));

        let id = insert_and_complete(&mut pool);
        insert_and_complete(&mut pool); // trigger purge by completing new task

        assert_eq!(pool.retrieve(&id), Some(Done(())));
    }

    fn insert_and_complete(pool: &mut TaskPool<(), EmptyProgress>) -> uuid::Uuid {
        let (handle, id) = pool.insert(EmptyProgress {});
        pool.complete(handle, ());
        id
    }

    fn get_inner_size<V, P>(pool: &TaskPool<V, P>) -> usize
    where
        P: Progressible,
    {
        pool.pending.len() + pool.completed.len()
    }
}
