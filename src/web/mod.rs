pub mod routes;

use crate::{config::Config, db::Database};
use axum::Router;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub cfg: Config,
}

pub fn create_router(db: Arc<Database>, cfg: Config) -> Router {
    routes::router(AppState { db, cfg })
}