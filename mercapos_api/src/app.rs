use crate::{
    app_state::AppState,
    presentation::{
        auth::require_auth,
        routes::{
            auth, catalogo, conteo_fisico, health::health, movimiento, operacion, producto, sync,
        },
        telemetry,
    },
};
use axum::{
    Router,
    http::{
        HeaderName, HeaderValue, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    middleware,
    routing::{get, post, put},
};
use tower_http::cors::CorsLayer;

fn capa_cors(origenes: &[String]) -> CorsLayer {
    let metodos = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ];
    let cabeceras = [
        CONTENT_TYPE,
        AUTHORIZATION,
        HeaderName::from_static("x-bootstrap-token"),
    ];
    let capa = CorsLayer::new()
        .allow_methods(metodos)
        .allow_headers(cabeceras);
    let permitidos = origenes
        .iter()
        .filter_map(|origen| origen.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();
    capa.allow_origin(permitidos)
}

pub fn build_app(state: AppState) -> Router {
    let telemetry_state = state.telemetry.clone();
    let publicas: Router<AppState> = Router::new()
        .route("/", get(|| async { "MercaPOS API funcionando" }))
        .route("/health", get(health))
        .route("/auth/login", post(auth::login))
        .route("/auth/bootstrap", post(auth::bootstrap));
    let protegidas: Router<AppState> = Router::new()
        .route("/metrics", get(telemetry::metrics))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/auth/password", post(auth::cambiar_password))
        .route("/productos", post(producto::crear).get(producto::listar))
        .route(
            "/productos/{id}",
            get(producto::obtener).put(producto::actualizar),
        )
        .route("/puestos", get(catalogo::listar_puestos))
        .route(
            "/puestos/{id}",
            get(catalogo::obtener_puesto).put(catalogo::actualizar_puesto),
        )
        .route(
            "/puestos/{puesto_id}/usuarios",
            get(catalogo::listar_usuarios),
        )
        .route("/usuarios", post(catalogo::crear_usuario))
        .route(
            "/usuarios/{id}",
            get(catalogo::obtener_usuario).put(catalogo::actualizar_usuario),
        )
        .route(
            "/operaciones",
            post(operacion::crear).get(operacion::listar),
        )
        .route("/operaciones/{id}", get(operacion::obtener))
        .route(
            "/conteos",
            post(conteo_fisico::crear).get(conteo_fisico::listar),
        )
        .route("/conteos/{id}", get(conteo_fisico::obtener))
        .route(
            "/movimientos",
            post(movimiento::crear).get(movimiento::listar),
        )
        .route("/puestos/{id}/existencias", get(movimiento::existencias))
        .route("/sync/operaciones", post(sync::sincronizar))
        .route("/sync/incidencias", get(sync::listar_incidencias))
        .route(
            "/sync/incidencias/{operacion_id}/resolver",
            put(sync::resolver_incidencia),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
    publicas
        .merge(protegidas)
        .with_state(state.clone())
        .layer(capa_cors(&state.cors_origins))
        .layer(middleware::from_fn_with_state(
            telemetry_state,
            telemetry::observe,
        ))
}
