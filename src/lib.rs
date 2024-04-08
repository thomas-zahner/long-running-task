use std::collections::HashMap;
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
    finished: HashMap<Uuid, T>,
}

impl<T, P> Default for TaskPool<T, P>
where
    P: Progressible,
{
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
            finished: HashMap::new(),
        }
    }
}

pub struct Handle {
    uuid: Uuid,
}

impl<T, P> TaskPool<T, P>
where
    P: Progressible + Clone,
{
    pub fn insert(&mut self, pending: P) -> (Handle, Uuid) {
        let uuid = Uuid::new_v4();
        self.pending.insert(uuid, pending);
        (Handle { uuid }, uuid)
    }

    /// Get the job state and remove it from the pool if it is done
    pub fn retrieve(&mut self, uuid: &Uuid) -> Option<TaskState<T, P>> {
        use TaskState::{Done, Pending};

        if let Some(p) = self.pending.get(uuid) {
            return Some(Pending(p.clone()));
        }

        self.finished.remove(uuid).map(|f| Done(f))
    }

    pub fn progress(&mut self, handle: &Handle) {
        self.pending
            .get_mut(&handle.uuid)
            .expect("Pending task not found. This should never happen because a task's handle cannot outlive the task.")
            .progress();
    }

    /// Mark the task associated to the handle as completed with a value of type T.
    /// The handle must be passed by value so that this is the final action.
    pub fn complete(&mut self, handle: Handle, value: T) {
        self.pending.remove(&handle.uuid);
        self.finished.insert(handle.uuid, value);
    }
}

#[cfg(test)]
mod tests {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Progress {
        pub progress: usize,
        pub total: usize,
    }

    impl Progressible for Progress {
        fn progress(&mut self) {
            self.progress = (self.progress + 1).min(self.total);
        }
    }

    use super::Progressible;
    use crate::{TaskPool, TaskState::*};

    #[test]
    fn insert_and_get() {
        let mut pool: TaskPool<u8, Progress> = Default::default();
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

    fn get_inner_size<T, P>(pool: &TaskPool<T, P>) -> usize
    where
        P: Progressible,
    {
        pool.pending.len() + pool.finished.len()
    }
}
