use crate::{
    app_state::AppState,
    application::auth::ahora_epoch,
    domain::{
        entities::{
            incidencia::{IncidenciaSincronizacion, TipoIncidencia, id_de_incidencia},
            movimiento::DireccionMovimiento,
            operacion::{Operacion, TipoDevolucion, TipoOperacion},
        },
        money::parse_scaled,
    },
    presentation::{
        auth::{AuthUser, error_repositorio, respuesta_error},
        routes::operacion::{OperacionRequest, convertir_autorizado, procesar},
    },
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const MAXIMO_OPERACIONES_POR_LOTE: usize = 100;

#[derive(Deserialize)]
pub struct SolicitudSync {
    pub operaciones: Vec<OperacionRequest>,
}

#[derive(Deserialize)]
pub struct FiltroIncidencias {
    pub puesto_id: Option<String>,
    pub resuelta: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct ResolucionRequest {
    pub nota: Option<String>,
}

fn incidencia_json(incidencia: &IncidenciaSincronizacion) -> Value {
    json!({
        "id": incidencia.id,
        "operacion_id": incidencia.operacion_id,
        "puesto_id": incidencia.puesto_id,
        "usuario_id": incidencia.usuario_id,
        "tipo": incidencia.tipo.as_str(),
        "codigo": incidencia.codigo,
        "motivo": incidencia.motivo,
        "intentos": incidencia.intentos,
        "creada_en": incidencia.creada_en,
        "actualizada_en": incidencia.actualizada_en,
        "resuelta": incidencia.resuelta,
        "resuelta_por": incidencia.resuelta_por,
        "resuelta_en": incidencia.resuelta_en,
        "nota": incidencia.nota,
    })
}

async fn registrar_incidencia(
    state: &AppState,
    auth: &AuthUser,
    operacion_id: &str,
    puesto_id: &str,
    tipo: TipoIncidencia,
    codigo: u16,
    motivo: String,
) -> Result<String, (StatusCode, Json<Value>)> {
    let ahora = ahora_epoch();
    let id = id_de_incidencia(operacion_id);
    let incidencia = IncidenciaSincronizacion {
        id: id.clone(),
        operacion_id: operacion_id.to_owned(),
        puesto_id: if puesto_id.is_empty() {
            auth.puesto_id.clone()
        } else {
            puesto_id.to_owned()
        },
        usuario_id: auth.id.clone(),
        tipo,
        codigo,
        motivo,
        intentos: 1,
        creada_en: ahora,
        actualizada_en: ahora,
        resuelta: false,
        resuelta_por: None,
        resuelta_en: None,
        nota: None,
        revision: None,
    };
    match state.incidencias.guardar(incidencia).await {
        Ok(()) => Ok(id),
        Err(error) => {
            tracing::error!(error=?error, operacion_id, "No se pudo registrar la incidencia; no se confirma el lote");
            Err(error_repositorio(error))
        }
    }
}

fn mensaje_de_cuerpo(cuerpo: &Json<Value>) -> String {
    cuerpo
        .0
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("Operación rechazada")
        .to_owned()
}

fn es_stock_insuficiente(cuerpo: &Json<Value>) -> bool {
    cuerpo.0.get("codigo").and_then(Value::as_str) == Some("stock_insuficiente")
}

async fn registrar_rechazo(
    state: &AppState,
    auth: &AuthUser,
    operacion_id: &str,
    puesto_id: &str,
    codigo: StatusCode,
    cuerpo: &Json<Value>,
) -> Result<(&'static str, String, Option<String>), (StatusCode, Json<Value>)> {
    let mensaje = mensaje_de_cuerpo(cuerpo);
    let incidente = registrar_incidencia(
        state,
        auth,
        operacion_id,
        puesto_id,
        TipoIncidencia::Rechazo,
        codigo.as_u16(),
        mensaje.clone(),
    )
    .await?;
    Ok(("rechazada", mensaje, Some(incidente)))
}

async fn advertencia_de_precio(state: &AppState, operacion: &Operacion) -> Option<String> {
    let compara_precio = matches!(operacion.tipo_operacion, TipoOperacion::Venta)
        || (matches!(operacion.tipo_operacion, TipoOperacion::Devolucion)
            && operacion.tipo_devolucion == Some(TipoDevolucion::Cliente));
    if !compara_precio {
        return None;
    }
    for renglon in &operacion.renglones {
        let Some(precio) = renglon.precio_unitario else {
            continue;
        };
        let Ok(Some(producto)) = state.productos.obtener_por_id(&renglon.producto_id).await else {
            continue;
        };
        let diferencia = producto.precio_unitario - precio;
        if diferencia.abs() > 0.00005 {
            return Some(format!(
                "El precio registrado para {} ({:.4}) difiere del precio actual ({:.4})",
                renglon.producto_id, precio, producto.precio_unitario
            ));
        }
    }
    None
}

async fn advertencia_de_stock(
    state: &AppState,
    operacion: &Operacion,
) -> Result<Option<String>, (StatusCode, Json<Value>)> {
    if !matches!(operacion.tipo_operacion, TipoOperacion::Venta) {
        return Ok(None);
    }
    let movimientos = state
        .movimientos
        .listar_por_puesto(&operacion.puesto_id)
        .await
        .map_err(error_repositorio)?;
    let mut disponible: HashMap<String, i64> = HashMap::new();
    for movimiento in movimientos {
        let cantidad = parse_scaled(&movimiento.cantidad.to_string(), 3, i64::MAX)
            .map_err(|mensaje| respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, mensaje))?;
        let firmado = if matches!(movimiento.direccion, DireccionMovimiento::Salida) {
            -cantidad
        } else {
            cantidad
        };
        let total = disponible.entry(movimiento.producto_id).or_default();
        *total = total.checked_add(firmado).ok_or_else(|| {
            respuesta_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Existencia fuera de rango",
            )
        })?;
    }
    let mut salida: HashMap<String, i64> = HashMap::new();
    for renglon in &operacion.renglones {
        let cantidad = parse_scaled(&renglon.cantidad.abs().to_string(), 3, 1_000_000_000)
            .map_err(|mensaje| respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, mensaje))?;
        let total = salida.entry(renglon.producto_id.clone()).or_default();
        *total = total.checked_add(cantidad).ok_or_else(|| {
            respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, "Cantidad fuera de rango")
        })?;
    }
    for (producto_id, cantidad) in salida {
        let despues = disponible.get(&producto_id).copied().unwrap_or_default();
        let antes = despues.checked_add(cantidad).ok_or_else(|| {
            respuesta_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Existencia fuera de rango",
            )
        })?;
        if antes < cantidad {
            return Ok(Some(format!(
                "Existencia insuficiente para {producto_id}: disponible {antes} milésimas, solicitada {cantidad} milésimas"
            )));
        }
    }
    Ok(None)
}

pub async fn sincronizar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(solicitud): Json<SolicitudSync>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if solicitud.operaciones.len() > MAXIMO_OPERACIONES_POR_LOTE {
        return Err(respuesta_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("Máximo {MAXIMO_OPERACIONES_POR_LOTE} operaciones por lote"),
        ));
    }
    let mut resultados = Vec::with_capacity(solicitud.operaciones.len());
    let mut aceptadas = 0_u32;
    let mut con_incidencia = 0_u32;
    let mut rechazadas = 0_u32;
    for solicitud_operacion in solicitud.operaciones {
        let operacion_id = solicitud_operacion.id.clone();
        let puesto_id = solicitud_operacion.puesto_id.clone();
        let (estado, mensaje, incidencia_id) = match convertir_autorizado(
            &auth,
            solicitud_operacion,
        ) {
            Err((codigo, cuerpo)) => {
                rechazadas += 1;
                registrar_rechazo(&state, &auth, &operacion_id, &puesto_id, codigo, &cuerpo).await?
            }
            Ok(operacion) => match procesar(&state, operacion.clone(), false).await {
                Ok((status, _)) => {
                    if let Some(motivo) = advertencia_de_precio(&state, &operacion).await {
                        let incidente = registrar_incidencia(
                            &state,
                            &auth,
                            &operacion_id,
                            &puesto_id,
                            TipoIncidencia::Advertencia,
                            StatusCode::OK.as_u16(),
                            motivo.clone(),
                        )
                        .await?;
                        con_incidencia += 1;
                        ("aceptada_con_incidencia", motivo, Some(incidente))
                    } else if status == StatusCode::OK
                        && state
                            .incidencias
                            .obtener_por_operacion(&operacion_id)
                            .await
                            .map_err(error_repositorio)?
                            .is_none()
                        && let Some(motivo_stock) = advertencia_de_stock(&state, &operacion).await?
                    {
                        let motivo = format!(
                            "Venta física aceptada con existencia insuficiente: {motivo_stock}"
                        );
                        let incidente = registrar_incidencia(
                            &state,
                            &auth,
                            &operacion_id,
                            &puesto_id,
                            TipoIncidencia::Advertencia,
                            StatusCode::OK.as_u16(),
                            motivo.clone(),
                        )
                        .await?;
                        con_incidencia += 1;
                        ("aceptada_con_incidencia", motivo, Some(incidente))
                    } else {
                        aceptadas += 1;
                        ("aceptada", "Operación aceptada".to_owned(), None)
                    }
                }
                Err((codigo, cuerpo)) => {
                    if matches!(operacion.tipo_operacion, TipoOperacion::Venta)
                        && es_stock_insuficiente(&cuerpo)
                    {
                        let motivo_stock = mensaje_de_cuerpo(&cuerpo);
                        match procesar(&state, operacion.clone(), true).await {
                            Ok(_) => {
                                let motivo = format!(
                                    "Venta física aceptada con existencia insuficiente: {motivo_stock}"
                                );
                                let incidente = registrar_incidencia(
                                    &state,
                                    &auth,
                                    &operacion_id,
                                    &puesto_id,
                                    TipoIncidencia::Advertencia,
                                    StatusCode::OK.as_u16(),
                                    motivo.clone(),
                                )
                                .await?;
                                con_incidencia += 1;
                                ("aceptada_con_incidencia", motivo, Some(incidente))
                            }
                            Err((codigo, cuerpo)) => {
                                rechazadas += 1;
                                registrar_rechazo(
                                    &state,
                                    &auth,
                                    &operacion_id,
                                    &puesto_id,
                                    codigo,
                                    &cuerpo,
                                )
                                .await?
                            }
                        }
                    } else {
                        rechazadas += 1;
                        registrar_rechazo(&state, &auth, &operacion_id, &puesto_id, codigo, &cuerpo)
                            .await?
                    }
                }
            },
        };
        resultados.push(json!({
            "operacion_id": operacion_id,
            "estado": estado,
            "mensaje": mensaje,
            "incidencia_id": incidencia_id,
        }));
    }
    Ok(Json(json!({
        "resultados": resultados,
        "aceptadas": aceptadas,
        "con_incidencia": con_incidencia,
        "rechazadas": rechazadas,
    })))
}

pub async fn listar_incidencias(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(filtro): Query<FiltroIncidencias>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if let Some(puesto) = &filtro.puesto_id {
        auth.exigir_puesto(puesto)?;
    }
    let puesto = Some(auth.puesto_id.clone());
    let incidencias = state
        .incidencias
        .listar(puesto.as_deref(), filtro.resuelta)
        .await
        .map_err(error_repositorio)?;
    Ok(Json(json!({
        "total": incidencias.len(),
        "incidencias": incidencias.iter().map(incidencia_json).collect::<Vec<_>>(),
    })))
}

pub async fn resolver_incidencia(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(operacion_id): Path<String>,
    Json(request): Json<ResolucionRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.exigir_administrador()?;
    let actual = state
        .incidencias
        .obtener_por_operacion(&operacion_id)
        .await
        .map_err(error_repositorio)?
        .ok_or_else(|| respuesta_error(StatusCode::NOT_FOUND, "Incidencia no encontrada"))?;
    auth.exigir_puesto(&actual.puesto_id)?;
    let incidencia = state
        .incidencias
        .resolver(&operacion_id, &auth.id, request.nota, ahora_epoch())
        .await
        .map_err(error_repositorio)?
        .ok_or_else(|| respuesta_error(StatusCode::NOT_FOUND, "Incidencia no encontrada"))?;
    Ok(Json(json!({"incidencia": incidencia_json(&incidencia)})))
}
