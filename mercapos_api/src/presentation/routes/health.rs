use axum::{Json, extract::State, http::StatusCode};

use serde_json::json;

use crate::app_state::AppState;

pub async fn health(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.couchdb.verificar_conexion().await {
        Ok(respuesta) => Ok(Json(json!({
            "api": "ok",
            "couchdb": respuesta
        }))),

        Err(error) => {
            tracing::error!(error=%error, "CouchDB health check failed");
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error":"CouchDB no está disponible"})),
            ))
        }
    }
}
