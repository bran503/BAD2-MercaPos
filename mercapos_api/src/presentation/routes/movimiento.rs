use crate::presentation::auth::{AuthUser, respuesta_error};
use crate::{
    app_state::AppState,
    application::use_cases::movimiento::{
        crear_movimiento::{CrearMovimiento, CrearMovimientoError},
        listar_movimientos_por_producto::ListarMovimientosPorProducto,
        listar_movimientos_por_puesto::ListarMovimientosPorPuesto,
    },
    domain::{
        entities::movimiento::{DireccionMovimiento, MotivoMovimiento, Movimiento},
        entities::usuario::Rol,
        money::parse_scaled,
        repositories::repository_error::RepositoryError,
    },
};
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub struct Filter {
    pub puesto_id: Option<String>,
    pub producto_id: Option<String>,
}
#[derive(Deserialize)]
pub struct Create {
    pub id: String,
    pub producto_id: String,
    pub puesto_id: String,
    pub cantidad: f64,
    pub motivo: String,
    pub dispositivo_id: String,
    pub usuario_id: String,
    pub fecha_local: String,
}
fn response(m: Movimiento) -> Value {
    json!({"id":m.id,"tipo":"movimiento","producto_id":m.producto_id,"puesto_id":m.puesto_id,"cantidad":if matches!(m.direccion,DireccionMovimiento::Salida){-m.cantidad.abs()}else{m.cantidad.abs()},"motivo":match m.motivo{crate::domain::entities::movimiento::MotivoMovimiento::InventarioInicial=>"inventario_inicial",crate::domain::entities::movimiento::MotivoMovimiento::Venta=>"venta",crate::domain::entities::movimiento::MotivoMovimiento::Compra=>"compra",crate::domain::entities::movimiento::MotivoMovimiento::Merma=>"merma",crate::domain::entities::movimiento::MotivoMovimiento::Devolucion=>"devolucion",crate::domain::entities::movimiento::MotivoMovimiento::Ajuste=>"ajuste"},"operacion_id":m.operacion_id,"usuario_id":m.usuario_id,"dispositivo_id":m.dispositivo_id,"fecha_local":m.fecha_local})
}
fn err(e: RepositoryError) -> (StatusCode, Json<Value>) {
    let (s, m) = match e {
        RepositoryError::NotFound => (StatusCode::NOT_FOUND, "No encontrado"),
        RepositoryError::AlreadyExists | RepositoryError::Conflict => {
            (StatusCode::CONFLICT, "Conflicto")
        }
        RepositoryError::ConnectionError(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "No se pudo conectar con CouchDB",
        ),
        RepositoryError::Unexpected(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Error interno"),
    };
    (s, Json(json!({"error":m})))
}
pub async fn listar(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(f): Query<Filter>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    if let Some(puesto) = &f.puesto_id {
        auth.exigir_puesto(puesto)?;
    } else if let Some(producto_id) = &f.producto_id {
        let producto = s
            .productos
            .obtener_por_id(producto_id)
            .await
            .map_err(err)?
            .ok_or_else(|| respuesta_error(StatusCode::NOT_FOUND, "Producto no encontrado"))?;
        auth.exigir_puesto(&producto.puesto_id)?;
    }
    let rows = match (f.producto_id, f.puesto_id) {
        (Some(producto), Some(puesto)) => ListarMovimientosPorPuesto::new(s.movimientos.clone())
            .ejecutar(&puesto)
            .await
            .map(|rows| {
                rows.into_iter()
                    .filter(|m| m.producto_id == producto)
                    .collect()
            }),
        (Some(p), None) => {
            ListarMovimientosPorProducto::new(s.movimientos.clone())
                .ejecutar(&p)
                .await
        }
        (None, Some(p)) => {
            ListarMovimientosPorPuesto::new(s.movimientos.clone())
                .ejecutar(&p)
                .await
        }
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"Indique puesto_id o producto_id"})),
            ));
        }
    }
    .map_err(err)?;
    Ok(Json(rows.into_iter().map(response).collect()))
}
pub async fn crear(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(i): Json<Create>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.exigir_administrador()?;
    auth.exigir_propietario(&i.usuario_id)?;
    auth.exigir_puesto(&i.puesto_id)?;
    let _guard = s.operation_lock.lock().await;
    let motivo = match i.motivo.as_str() {
        "inventario_inicial" => MotivoMovimiento::InventarioInicial,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"error":"Solo se pueden crear movimientos de inventario_inicial de forma directa; ventas, compras y ajustes se generan desde /operaciones"}),
                ),
            ));
        }
    };
    let puesto = s.puestos.obtener_por_id(&i.puesto_id).await.map_err(err)?;
    if puesto.as_ref().is_none_or(|p| !p.activo) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"El puesto no existe o está inactivo"})),
        ));
    }
    let producto = s
        .productos
        .obtener_por_id(&i.producto_id)
        .await
        .map_err(err)?;
    if producto
        .as_ref()
        .is_none_or(|p| !p.activo || p.puesto_id != i.puesto_id)
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"El producto no existe o está inactivo"})),
        ));
    }
    let usuario = s
        .usuarios
        .obtener_por_id(&i.usuario_id)
        .await
        .map_err(err)?;
    if usuario
        .as_ref()
        .is_none_or(|u| !u.activo || u.rol != Rol::Administrador)
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"La apertura exige un administrador activo del puesto"})),
        ));
    }
    let m = Movimiento {
        id: i.id,
        producto_id: i.producto_id,
        puesto_id: i.puesto_id,
        cantidad: i.cantidad,
        motivo,
        operacion_id: None,
        usuario_id: Some(i.usuario_id),
        dispositivo_id: i.dispositivo_id,
        fecha_local: i.fecha_local,
        direccion: DireccionMovimiento::Entrada,
    };
    let prior = s
        .movimientos
        .listar_por_puesto(&m.puesto_id)
        .await
        .map_err(err)?;
    let existing = prior.iter().find(|row| row.producto_id == m.producto_id);
    if let Some(row) = existing {
        if row == &m
            && prior
                .iter()
                .filter(|x| x.producto_id == m.producto_id)
                .count()
                == 1
        {
            return Ok((StatusCode::OK, Json(response(m))));
        }
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error":"El producto ya tiene historial de inventario en este puesto"})),
        ));
    }
    CrearMovimiento::new(s.movimientos.clone())
        .ejecutar(m.clone())
        .await
        .map_err(|e| match e {
            CrearMovimientoError::DatosInvalidos(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error":msg})))
            }
            CrearMovimientoError::Repositorio(e) => err(e),
        })?;
    Ok((StatusCode::CREATED, Json(response(m))))
}
pub async fn existencias(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    axum::extract::Path(puesto_id): axum::extract::Path<String>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    auth.exigir_puesto(&puesto_id)?;
    let rows = ListarMovimientosPorPuesto::new(s.movimientos.clone())
        .ejecutar(&puesto_id)
        .await
        .map_err(err)?;
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for m in rows {
        let amount = parse_scaled(&m.cantidad.to_string(), 3, i64::MAX).map_err(|_| {
            err(RepositoryError::Unexpected(
                "Cantidad de movimiento fuera de escala".into(),
            ))
        })?;
        let q = if matches!(m.direccion, DireccionMovimiento::Salida) {
            -amount
        } else {
            amount
        };
        let prior = totals.get(&m.producto_id).copied().unwrap_or_default();
        let total = prior.checked_add(q).ok_or_else(|| {
            err(RepositoryError::Unexpected(
                "Existencia fuera de rango".into(),
            ))
        })?;
        totals.insert(m.producto_id, total);
    }
    Ok(Json(totals.into_iter().map(|(producto_id,cantidad_mil)|json!({"puesto_id":puesto_id,"producto_id":producto_id,"existencia":cantidad_mil as f64 / 1000.0,"existencia_mil":cantidad_mil})).collect()))
}
