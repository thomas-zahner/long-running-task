use std::time::Instant;
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

pub trait Progressible {
    fn progress(&mut self);
}

#[derive(PartialEq, Eq, Debug)]
pub enum TaskState<T, P>
where
    P: Progressible,
{
    Pending(P),
    Done(T),
}

pub struct TaskPool<T, P>
where
    P: Progressible,
{
    pending: HashMap<Uuid, P>,
    completed: HashMap<Uuid, (Instant, T)>,
    max_lifespan: Option<Duration>,
}

impl<T, P> Default for TaskPool<T, P>
where
    P: Progressible,
{
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
            completed: HashMap::new(),
            max_lifespan: None,
        }
    }
}

/// A unique handle to a single task.
/// Does not implement clone or expose its inner fields
/// because it must be a unique reference to the task.
pub struct Handle {
    uuid: Uuid,
}

impl<T, P> TaskPool<T, P>
where
    P: Progressible + Clone,
{
    /// Configure the maximum lifespan of tasks.
    /// `None` means that tasks will never expire.
    /// Expired tasks are purged as soon as `complete` is invoked.
    #[cfg(feature = "lifespan")]
    pub fn with_max_lifespan(mut self, max_lifespan: Option<Duration>) -> Self {
        self.max_lifespan = max_lifespan;
        self
    }

    pub fn insert(&mut self, pending: P) -> (Handle, Uuid) {
        let uuid = Uuid::new_v4();
        self.pending.insert(uuid, pending);
        (Handle { uuid }, uuid)
    }

    /// Get the task state and remove it from the pool if it is done
    pub fn retrieve(&mut self, uuid: &Uuid) -> Option<TaskState<T, P>> {
        use TaskState::{Done, Pending};

        if let Some(p) = self.pending.get(uuid) {
            return Some(Pending(p.clone()));
        }

        self.completed.remove(uuid).map(|f| Done(f.1))
    }

    pub fn progress(&mut self, handle: &Handle) {
        self.pending
            .get_mut(&handle.uuid)
            .expect("Pending task not found. This should never happen because a task's handle cannot outlive the task.")
            .progress();
    }

    /// Mark the task associated to the handle as completed with a value of type T.
    /// The handle must be passed by value so that this is the final action.
    /// As a side effect expired tasks are purged.
    pub fn complete(&mut self, handle: Handle, value: T) {
        self.pending.remove(&handle.uuid);
        self.purge_expired_tasks();
        self.completed.insert(handle.uuid, (Instant::now(), value));
    }

    fn purge_expired_tasks(&mut self) {
        if let Some(max_lifespan) = self.max_lifespan {
            let now = Instant::now();
            self.completed
                .retain(|_, (inserted_at, _)| now.duration_since(*inserted_at) < max_lifespan);
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
    use crate::{
        TaskPool,
        TaskState::{self, *},
    };

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
    fn lifespan() {
        let lifespan = Duration::from_millis(10);
        let mut pool = TaskPool::<(), EmptyProgress>::default().with_max_lifespan(Some(lifespan));

        let (handle, id) = pool.insert(EmptyProgress {});
        pool.complete(handle, ());
        assert_eq!(pool.retrieve(&id), Some(TaskState::Done(())));

        let (handle, id) = pool.insert(EmptyProgress {});
        pool.complete(handle, ());

        thread::sleep(lifespan); // exceed time

        assert_eq!(get_inner_size(&pool), 1);

        // trigger purge by completing new task
        let h = pool.insert(EmptyProgress {}).0;
        pool.complete(h, ()); // trigger purge

        assert_eq!(pool.retrieve(&id), None);
        assert_eq!(get_inner_size(&pool), 1);
    }

    fn get_inner_size<T, P>(pool: &TaskPool<T, P>) -> usize
    where
        P: Progressible,
    {
        pool.pending.len() + pool.completed.len()
    }
}
