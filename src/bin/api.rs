// SPDX-License-Identifier: GPL-3.0-or-later

use aliasable::prelude::AliasableBox;
use anyhow::Result;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

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

// see also: https://morestina.net/blog/1868/self-referential-types-for-fun-and-profit
struct AsyncDbResponse {
    // actually has lifetime of `db`
    // declared first so it's droped before `db`
    stream: StreamBodyAs<'static>,
    #[allow(dead_code)]
    db: AliasableBox<sea_orm::DatabaseConnection>,
}

impl IntoResponse for AsyncDbResponse {
    fn into_response(self) -> Response {
        self.stream.into_response()
    }
}

fn to_aliasable(db: sea_orm::DatabaseConnection) -> AliasableBox<sea_orm::DatabaseConnection> {
    AliasableBox::from_unique(Box::new(db))
}

async fn power_today(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let db = to_aliasable(state.db.clone());
    let db_stream = db::select_power_today(db.as_ref()).await?;
    let json_stream = StreamBodyAsOptions::new()
        .buffering_ready_items(1000)
        .json_array(db_stream);
    let json_stream = unsafe { std::mem::transmute(json_stream) };
    Ok(AsyncDbResponse {
        stream: json_stream,
        db,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let db = db::setup();
    let shared_state = Arc::new(AppState { db: db.await? });

    // build our application with a single route
    let app = Router::new()
        .route("/powerToday", get(power_today))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    // run our app with hyper, listening locally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4334").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
