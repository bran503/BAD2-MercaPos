use crate::presentation::auth::AuthUser;
use crate::{
    app_state::AppState,
    application::use_cases::conteo_fisico::{
        crear_conteo::CrearConteo, listar_conteos_por_puesto::ListarConteosPorPuesto,
        obtener_conteo::ObtenerConteo,
    },
    domain::entities::conteo_fisico::{ConteoFisico, RenglonConteo},
    domain::entities::movimiento::DireccionMovimiento,
    domain::money::parse_scaled,
    domain::repositories::repository_error::RepositoryError,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
#[derive(Deserialize)]
pub struct Request {
    pub id: String,
    pub puesto_id: String,
    pub usuario_id: String,
    pub dispositivo_id: String,
    pub fecha_local: String,
    pub periodo_estudio: String,
    pub etapa: String,
    pub origen_registro: String,
    pub evidencia: Option<String>,
    pub renglones: Vec<Row>,
}
#[derive(Deserialize)]
pub struct Row {
    pub producto_id: String,
    pub cantidad_sistema: Option<f64>,
    pub cantidad_contada: f64,
    pub diferencia: Option<f64>,
}
#[derive(Deserialize)]
pub struct Filter {
    pub puesto_id: String,
}
fn response(c: ConteoFisico) -> Value {
    json!({"id":c.id,"tipo":"conteo_fisico","puesto_id":c.puesto_id,"usuario_id":c.usuario_id,"dispositivo_id":c.dispositivo_id,"fecha_local":c.fecha_local,"periodo_estudio":c.periodo_estudio,"etapa":c.etapa,"origen_registro":c.origen_registro,"evidencia":c.evidencia,"corte_seq":c.corte_seq,"renglones":c.renglones.iter().map(|r|json!({"producto_id":r.producto_id,"cantidad_sistema":r.cantidad_sistema,"cantidad_contada":r.cantidad_contada,"diferencia":r.diferencia})).collect::<Vec<_>>()})
}
fn error(e: RepositoryError) -> (StatusCode, Json<Value>) {
    let (s, m) = match e {
        RepositoryError::NotFound => (StatusCode::NOT_FOUND, "Recurso no encontrado"),
        RepositoryError::AlreadyExists => (StatusCode::CONFLICT, "El identificador ya existe"),
        RepositoryError::Conflict => (StatusCode::CONFLICT, "Conflicto en CouchDB"),
        RepositoryError::ConnectionError(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "No se pudo conectar con CouchDB",
        ),
        RepositoryError::Unexpected(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Error interno"),
    };
    (s, Json(json!({"error":m})))
}
pub async fn crear(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(r): Json<Request>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.exigir_propietario(&r.usuario_id)?;
    auth.exigir_puesto(&r.puesto_id)?;
    let _guard = s.operation_lock.lock().await;
    let mut stock: HashMap<String, i64> = HashMap::new();
    let corte_seq = if r.origen_registro == "pos" {
        let movements = s
            .movimientos
            .listar_por_puesto(&r.puesto_id)
            .await
            .map_err(error)?;
        for movement in movements {
            let quantity =
                parse_scaled(&movement.cantidad.to_string(), 3, i64::MAX).map_err(|message| {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(json!({"error":message})),
                    )
                })?;
            let signed = if movement.direccion == DireccionMovimiento::Salida {
                -quantity
            } else {
                quantity
            };
            let before = stock
                .get(&movement.producto_id)
                .copied()
                .unwrap_or_default();
            let after = before.checked_add(signed).ok_or_else(|| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"Existencia fuera de rango"})),
                )
            })?;
            stock.insert(movement.producto_id, after);
        }
        let info: Value = s.couchdb.get(s.couchdb.database()).await.map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error":"No se pudo obtener el corte de la base"})),
            )
        })?;
        Some(
            info.get("update_seq")
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string())
                })
                .ok_or_else(|| {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({"error":"La base no informó update_seq"})),
                    )
                })?,
        )
    } else {
        None
    };
    let mut rows = Vec::with_capacity(r.renglones.len());
    for row in r.renglones {
        let system = if r.origen_registro == "pos" {
            let value = stock.get(&row.producto_id).copied().unwrap_or_default();
            if let Some(sent) = row.cantidad_sistema {
                let sent = parse_scaled(&sent.to_string(), 3, i64::MAX).map_err(|message| {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(json!({"error":message})),
                    )
                })?;
                if sent != value {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(
                            json!({"error":"cantidad_sistema no coincide con el corte del servidor"}),
                        ),
                    ));
                }
            }
            value
        } else {
            let sent = row.cantidad_sistema.ok_or_else(|| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"cantidad_sistema manual es obligatoria"})),
                )
            })?;
            parse_scaled(&sent.to_string(), 3, i64::MAX).map_err(|message| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":message})),
                )
            })?
        };
        let counted =
            parse_scaled(&row.cantidad_contada.to_string(), 3, i64::MAX).map_err(|message| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":message})),
                )
            })?;
        let difference = counted.checked_sub(system).ok_or_else(|| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"error":"Diferencia fuera de rango"})),
            )
        })?;
        if let Some(sent) = row.diferencia {
            let sent = parse_scaled(&sent.to_string(), 3, i64::MAX).map_err(|message| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":message})),
                )
            })?;
            if sent != difference {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"La diferencia no coincide"})),
                ));
            }
        }
        rows.push(RenglonConteo {
            producto_id: row.producto_id,
            cantidad_sistema: system as f64 / 1000.0,
            cantidad_contada: counted as f64 / 1000.0,
            diferencia: difference as f64 / 1000.0,
        });
    }
    let c = ConteoFisico {
        id: r.id,
        puesto_id: r.puesto_id,
        usuario_id: r.usuario_id,
        dispositivo_id: r.dispositivo_id,
        fecha_local: r.fecha_local,
        periodo_estudio: Some(r.periodo_estudio),
        etapa: Some(r.etapa),
        origen_registro: Some(r.origen_registro),
        evidencia: r.evidencia,
        corte_seq,
        renglones: rows,
    };
    let puesto = s
        .puestos
        .obtener_por_id(&c.puesto_id)
        .await
        .map_err(error)?;
    if puesto.as_ref().is_none_or(|p| !p.activo) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"El puesto no existe o está inactivo"})),
        ));
    }
    let usuario = s
        .usuarios
        .obtener_por_id(&c.usuario_id)
        .await
        .map_err(error)?;
    if usuario.as_ref().is_none_or(|u| !u.activo) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"El usuario no existe, está inactivo o no pertenece al puesto"})),
        ));
    }
    for row in &c.renglones {
        let producto = s
            .productos
            .obtener_por_id(&row.producto_id)
            .await
            .map_err(error)?;
        if producto
            .as_ref()
            .is_none_or(|p| !p.activo || p.puesto_id != c.puesto_id)
        {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(
                    json!({"error":format!("El producto {} no existe o está inactivo",row.producto_id)}),
                ),
            ));
        }
    }
    CrearConteo::new(s.conteos.clone()).ejecutar(c.clone()).await.map_err(|e|match e{crate::application::use_cases::conteo_fisico::crear_conteo::CrearConteoError::DatosInvalidos(m)=>(StatusCode::UNPROCESSABLE_ENTITY,Json(json!({"error":m}))),crate::application::use_cases::conteo_fisico::crear_conteo::CrearConteoError::Repositorio(e)=>error(e)})?;
    Ok((StatusCode::CREATED, Json(response(c))))
}
pub async fn obtener(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match ObtenerConteo::new(s.conteos.clone())
        .ejecutar(&id)
        .await
        .map_err(error)?
    {
        Some(c) => {
            auth.exigir_puesto(&c.puesto_id)?;
            Ok(Json(response(c)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Conteo no encontrado"})),
        )),
    }
}
pub async fn listar(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(f): Query<Filter>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    auth.exigir_puesto(&f.puesto_id)?;
    let cs = ListarConteosPorPuesto::new(s.conteos.clone())
        .ejecutar(&f.puesto_id)
        .await
        .map_err(error)?;
    Ok(Json(cs.into_iter().map(response).collect()))
}
