use std::sync::Arc;

use crate::application::auth::LimitadorLogin;
use crate::domain::repositories::auth_repository::AuthRepository;
use crate::domain::repositories::incidencia_repository::IncidenciaRepository;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::{
    conteo_repository::ConteoRepository, movimiento_repository::MovimientoRepository,
    operacion_repository::OperacionRepository, puesto_repository::PuestoRepository,
    usuario_repository::UsuarioRepository,
};
use crate::infrastructure::couchdb::client::CouchDbClient;
use crate::presentation::telemetry::Telemetry;

pub const TTL_SESION_SEGUNDOS: i64 = 8 * 60 * 60;

#[derive(Clone)]
pub struct AppState {
    pub couchdb: CouchDbClient,
    pub productos: Arc<dyn ProductoRepository>,
    pub movimientos: Arc<dyn MovimientoRepository>,
    pub operaciones: Arc<dyn OperacionRepository>,
    pub conteos: Arc<dyn ConteoRepository>,
    pub puestos: Arc<dyn PuestoRepository>,
    pub usuarios: Arc<dyn UsuarioRepository>,
    pub auth: Arc<dyn AuthRepository>,
    pub incidencias: Arc<dyn IncidenciaRepository>,
    pub telemetry: Arc<Telemetry>,
    pub operation_lock: Arc<tokio::sync::Mutex<()>>,
    pub bootstrap_token: Option<String>,
    pub bootstrap_lock: Arc<tokio::sync::Mutex<()>>,
    pub login_limiter: Arc<LimitadorLogin>,
    pub session_ttl_seconds: i64,
    pub cors_origins: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
impl AppState {
    pub fn new(
        couchdb: CouchDbClient,
        productos: Arc<dyn ProductoRepository>,
        movimientos: Arc<dyn MovimientoRepository>,
        operaciones: Arc<dyn OperacionRepository>,
        conteos: Arc<dyn ConteoRepository>,
        puestos: Arc<dyn PuestoRepository>,
        usuarios: Arc<dyn UsuarioRepository>,
        auth: Arc<dyn AuthRepository>,
        incidencias: Arc<dyn IncidenciaRepository>,
    ) -> Self {
        Self {
            couchdb,
            productos,
            movimientos,
            operaciones,
            conteos,
            puestos,
            usuarios,
            auth,
            incidencias,
            telemetry: Arc::new(Telemetry::default()),
            operation_lock: Arc::new(tokio::sync::Mutex::new(())),
            bootstrap_token: None,
            bootstrap_lock: Arc::new(tokio::sync::Mutex::new(())),
            login_limiter: Arc::new(LimitadorLogin::default()),
            session_ttl_seconds: TTL_SESION_SEGUNDOS,
            cors_origins: vec![
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:5173".to_string(),
            ],
        }
    }

    pub fn con_configuracion(
        mut self,
        bootstrap_token: Option<String>,
        session_ttl_seconds: i64,
        cors_origins: Vec<String>,
    ) -> Self {
        self.bootstrap_token = bootstrap_token;
        self.session_ttl_seconds = session_ttl_seconds.max(60);
        self.cors_origins = cors_origins;
        self
    }
}
