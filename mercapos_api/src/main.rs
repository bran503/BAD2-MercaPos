use backend::{
    app::build_app,
    app_state::{AppState, TTL_SESION_SEGUNDOS},
    infrastructure::couchdb::{
        client::CouchDbClient,
        repositories::{
            auth_repository::CouchDbAuthRepository, catalogo_repository::CouchDbCatalogoRepository,
            conteo_repository::CouchDbConteoRepository,
            incidencia_repository::CouchDbIncidenciaRepository,
            movimiento_repository::CouchDbMovimientoRepository,
            operacion_repository::CouchDbOperacionRepository,
            producto_repository::CouchDbProductoRepository,
        },
    },
};
use std::{env, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();
    let couchdb = CouchDbClient::new(
        env::var("COUCHDB_URL")?,
        env::var("COUCHDB_USERNAME")?,
        env::var("COUCHDB_PASSWORD")?,
        env::var("COUCHDB_DATABASE")?,
    )?;
    let auth_couchdb = CouchDbClient::new(
        env::var("COUCHDB_URL")?,
        env::var("COUCHDB_USERNAME")?,
        env::var("COUCHDB_PASSWORD")?,
        env::var("COUCHDB_AUTH_DATABASE").unwrap_or_else(|_| "mercapos_auth".into()),
    )?;
    let catalogos = Arc::new(CouchDbCatalogoRepository::new(couchdb.clone()));
    let state = AppState::new(
        couchdb.clone(),
        Arc::new(CouchDbProductoRepository::new(couchdb.clone())),
        Arc::new(CouchDbMovimientoRepository::new(couchdb.clone())),
        Arc::new(CouchDbOperacionRepository::new(couchdb.clone())),
        Arc::new(CouchDbConteoRepository::new(couchdb.clone())),
        catalogos.clone(),
        catalogos,
        Arc::new(CouchDbAuthRepository::new(auth_couchdb)),
        Arc::new(CouchDbIncidenciaRepository::new(couchdb.clone())),
    )
    .con_configuracion(
        env::var("MERCAPOS_BOOTSTRAP_TOKEN")
            .ok()
            .filter(|token| !token.trim().is_empty()),
        env::var("MERCAPOS_SESSION_TTL_SECONDS")
            .ok()
            .and_then(|valor| valor.parse().ok())
            .unwrap_or(TTL_SESION_SEGUNDOS),
        env::var("MERCAPOS_CORS_ORIGINS")
            .map(|origenes| {
                origenes
                    .split(',')
                    .map(str::trim)
                    .filter(|origen| !origen.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_else(|_| {
                vec![
                    "http://localhost:5173".to_string(),
                    "http://127.0.0.1:5173".to_string(),
                ]
            }),
    );
    let app = build_app(state);
    let address = env::var("API_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(address=%address,"MercaPOS API iniciada");
    axum::serve(listener, app).await?;
    Ok(())
}
