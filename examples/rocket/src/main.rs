use long_running_task::{Progressible, TaskPool, TaskState};
use rocket::{
    get, launch, post, routes,
    serde::{json::Json, uuid::Uuid, Serialize},
    State,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{task, time::sleep};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Response(usize);

#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Progress {
    pub progress: usize,
    pub total: usize,
}

impl Progressible for Progress {
    fn progress(&mut self) {
        self.progress = (self.progress + 1).min(self.total);
    }
}

/// curl -X POST http://localhost:8000
#[post("/")]
fn start_task(task_pool: &State<Arc<Mutex<TaskPool<Response, Progress>>>>) -> String {
    let total = 10;
    let (handle, uuid) = task_pool
        .lock()
        .unwrap()
        .insert(Progress { progress: 0, total });

    let task_pool = Arc::clone(task_pool);

    task::spawn(async move {
        for _ in 0..total {
            sleep(Duration::from_millis(1_000)).await;
            task_pool.lock().unwrap().progress(&handle);
        }

        task_pool.lock().unwrap().complete(handle, Response(42));
    });

    uuid.to_string()
}

/// curl http://localhost:8000/<uuid>
#[get("/<uuid>")]
fn get_task(
    uuid: Uuid,
    task_pool: &State<Arc<Mutex<TaskPool<Response, Progress>>>>,
) -> Option<Json<TaskState<Response, Progress>>> {
    task_pool.lock().unwrap().retrieve(&uuid).map(Json)
}

#[launch]
fn rocket() -> _ {
    let task_pool =
        TaskPool::<Response, Progress>::default().with_lifespan(Some(Duration::from_secs(60)));

    rocket::build()
        .mount("/", routes![start_task, get_task])
        .manage(Arc::new(Mutex::new(task_pool)))
}
