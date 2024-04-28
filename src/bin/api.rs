// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use futures::Stream;
use serde::Serialize;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use axum_streams::*;

use powerlog::db;

struct AppState {
    db: sea_orm::DatabaseConnection,
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Clone, Serialize)]
struct PowerToday {
    time: String,
    out1: f32,
    produced1: f32,
    out2: f32,
    produced2: f32,
}

// Your possibly stream of objects
fn power_today_stream() -> impl Stream<Item = PowerToday> {
    // Simulating a stream with a plain vector
    futures::stream::iter(vec![
        PowerToday {
            time: "Asdf".into(),
            out1: 0.1,
            produced1: 1.,
            out2: 0.2,
            produced2: 2.,
        };
        1000
    ])
}

async fn power_today(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    StreamBodyAsOptions::new()
        .buffering_ready_items(1000)
        .json_array(power_today_stream())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let db = db::setup();
    let shared_state = Arc::new(AppState { db: db.await? });

    // build our application with a single route
    let app = Router::new()
        .route("/powerToday", get(power_today))
        .with_state(shared_state);

    // run our app with hyper, listening locally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4334").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
