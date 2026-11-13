pub mod routes;

use crate::{RuntimeStatus, config::Config, db::Database};
use axum::Router;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub cfg: Arc<RwLock<Config>>,
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
    cfg: Arc<RwLock<Config>>,
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
