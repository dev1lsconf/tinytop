pub mod routes;

use crate::{config::Config, db::Database, RuntimeStatus};
use axum::Router;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub cfg: Config,
    pub runtime: Arc<RwLock<RuntimeStatus>>,
    pub shutdown: Arc<AtomicBool>,
}

impl AppState {
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

pub fn create_router(
    db: Arc<Database>,
    cfg: Config,
    runtime: Arc<RwLock<RuntimeStatus>>,
    shutdown: Arc<AtomicBool>,
) -> Router {
    routes::router(AppState {
        db,
        cfg,
        runtime,
        shutdown,
    })
}
