use crate::domain::money::{parse_scaled, subtotal_centavos};
use crate::presentation::auth::AuthUser;
use crate::{
    app_state::AppState,
    application::use_cases::operacion::crear_operacion::CrearOperacionError,
    application::use_cases::operacion::{
        crear_operacion::CrearOperacion, listar_operaciones_por_puesto::ListarOperacionesPorPuesto,
        obtener_operacion::ObtenerOperacion,
    },
    domain::entities::operacion::{Operacion, RenglonOperacion, TipoDevolucion, TipoOperacion},
    domain::repositories::repository_error::RepositoryError,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Number, Value, json};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct OperacionRequest {
    pub id: String,
    pub tipo_operacion: String,
    pub puesto_id: String,
    pub usuario_id: String,
    pub dispositivo_id: String,
    pub fecha_local: String,
    pub renglones: Vec<RenglonRequest>,
    pub total: Option<Number>,
    pub motivo: Option<String>,
    pub operacion_origen_id: Option<String>,
    pub conteo_id: Option<String>,
    pub tipo_devolucion: Option<String>,
    pub destino_devolucion: Option<String>,
    pub pago: Option<PagoRequest>,
}
#[derive(Deserialize)]
pub struct PagoRequest {
    pub metodo: String,
    pub modo: String,
    pub estado: String,
    pub autorizacion: String,
    pub fecha: String,
}
#[derive(Deserialize)]
pub struct RenglonRequest {
    pub producto_id: String,
    pub cantidad: Number,
    pub precio_unitario: Option<Number>,
    pub costo_unitario: Option<Number>,
}
#[derive(Deserialize)]
pub struct FiltroPuesto {
    pub puesto_id: String,
}
pub(crate) fn convertir(r: OperacionRequest) -> Result<Operacion, String> {
    let tipo_operacion = match r.tipo_operacion.as_str() {
        "venta" => TipoOperacion::Venta,
        "compra" => TipoOperacion::Compra,
        "merma" => TipoOperacion::Merma,
        "devolucion" => TipoOperacion::Devolucion,
        "ajuste" => TipoOperacion::Ajuste,
        _ => {
            return Err("tipo_operacion debe ser venta, compra, merma, devolucion o ajuste".into());
        }
    };
    let tipo_devolucion = match r.tipo_devolucion.as_deref() {
        Some("cliente") => Some(TipoDevolucion::Cliente),
        Some("proveedor") => Some(TipoDevolucion::Proveedor),
        None => None,
        Some(_) => return Err("tipo_devolucion debe ser cliente o proveedor".into()),
    };
    if r.renglones.len() > 100 {
        return Err("Máximo 100 renglones por operación".into());
    }
    let mut calculated_cents = 0_i64;
    let mut lines = Vec::with_capacity(r.renglones.len());
    for line in r.renglones {
        let quantity = parse_scaled(&line.cantidad.to_string(), 3, 1_000_000_000)?;
        let price = line
            .precio_unitario
            .as_ref()
            .map(|n| parse_scaled(&n.to_string(), 4, 10_000_000_000))
            .transpose()?;
        let cost = line
            .costo_unitario
            .as_ref()
            .map(|n| parse_scaled(&n.to_string(), 4, 10_000_000_000))
            .transpose()?;
        if let Some(unit) = match tipo_operacion {
            TipoOperacion::Venta => price,
            TipoOperacion::Devolucion if tipo_devolucion == Some(TipoDevolucion::Proveedor) => cost,
            TipoOperacion::Devolucion => price,
            TipoOperacion::Compra => cost,
            _ => None,
        } {
            if quantity < 0 || unit < 0 {
                return Err("Cantidad y precio deben ser positivos".into());
            }
            let cents = subtotal_centavos(quantity, unit)?;
            calculated_cents = calculated_cents
                .checked_add(cents)
                .ok_or("Importe fuera de rango")?;
        }
        lines.push(RenglonOperacion {
            producto_id: line.producto_id,
            cantidad: quantity as f64 / 1000.0,
            precio_unitario: price.map(|v| v as f64 / 10_000.0),
            costo_unitario: cost.map(|v| v as f64 / 10_000.0),
            nombre_producto: None,
            unidad: None,
        });
    }
    let monetary = matches!(
        tipo_operacion,
        TipoOperacion::Venta | TipoOperacion::Compra | TipoOperacion::Devolucion
    );
    if calculated_cents > 10_000_000_000 {
        return Err("Importe fuera de rango".into());
    }
    let sent_cents = r
        .total
        .as_ref()
        .map(|n| parse_scaled(&n.to_string(), 2, 10_000_000_000))
        .transpose()?;
    if monetary && sent_cents.is_some_and(|sent| sent != calculated_cents) {
        return Err(format!(
            "Total enviado {} centavos; calculado {} centavos",
            sent_cents.unwrap_or_default(),
            calculated_cents
        ));
    }
    if !monetary && sent_cents.is_some() {
        return Err("Este tipo de operación no lleva total".into());
    }
    let total_centavos = if monetary {
        Some(calculated_cents)
    } else {
        None
    };
    let pago = match r.pago {
        None => None,
        Some(p) => {
            if tipo_operacion != TipoOperacion::Venta {
                return Err("Solo las ventas admiten pago".into());
            }
            if p.metodo != "tarjeta" || p.modo != "simulado" {
                return Err("El pago debe ser simulado con tarjeta".into());
            }
            if !["aprobado", "rechazado", "cancelado"].contains(&p.estado.as_str()) {
                return Err("estado de pago inválido".into());
            }
            if p.autorizacion.trim().is_empty() || p.fecha.trim().is_empty() {
                return Err("Pago sin autorización o fecha".into());
            }
            Some(crate::domain::entities::operacion::Pago {
                metodo: p.metodo,
                modo: p.modo,
                estado: p.estado,
                autorizacion: p.autorizacion,
                fecha: p.fecha,
            })
        }
    };
    Ok(Operacion {
        id: r.id,
        tipo_operacion,
        puesto_id: r.puesto_id,
        usuario_id: r.usuario_id,
        dispositivo_id: r.dispositivo_id,
        fecha_local: r.fecha_local,
        recibido_en_epoch_ms: None,
        renglones: lines,
        total: total_centavos.map(|c| c as f64 / 100.0),
        total_centavos,
        motivo: r.motivo,
        operacion_origen_id: r.operacion_origen_id,
        conteo_id: r.conteo_id,
        tipo_devolucion,
        destino_devolucion: r.destino_devolucion,
        pago,
    })
}
fn respuesta(o: Operacion) -> Value {
    json!({"id":o.id,"tipo":"operacion","tipo_operacion":match o.tipo_operacion{TipoOperacion::Venta=>"venta",TipoOperacion::Compra=>"compra",TipoOperacion::Merma=>"merma",TipoOperacion::Devolucion=>"devolucion",TipoOperacion::Ajuste=>"ajuste"},"puesto_id":o.puesto_id,"usuario_id":o.usuario_id,"dispositivo_id":o.dispositivo_id,"fecha_local":o.fecha_local,"recibido_en_epoch_ms":o.recibido_en_epoch_ms,"renglones":o.renglones.iter().map(|x|json!({"producto_id":x.producto_id,"nombre_producto":x.nombre_producto,"unidad":x.unidad,"cantidad":x.cantidad,"precio_unitario":x.precio_unitario,"costo_unitario":x.costo_unitario})).collect::<Vec<_>>(),"total":o.total,"total_centavos":o.total_centavos,"motivo":o.motivo,"operacion_origen_id":o.operacion_origen_id,"conteo_id":o.conteo_id,"tipo_devolucion":o.tipo_devolucion.map(|x|match x{TipoDevolucion::Cliente=>"cliente",TipoDevolucion::Proveedor=>"proveedor"}),"destino_devolucion":o.destino_devolucion,"pago":o.pago.map(|p|json!({"metodo":p.metodo,"modo":p.modo,"estado":p.estado,"autorizacion":p.autorizacion,"fecha":p.fecha}))})
}
fn map_error(e: RepositoryError) -> (StatusCode, Json<Value>) {
    let (s, m) = match e {
        RepositoryError::NotFound => (StatusCode::NOT_FOUND, "Recurso no encontrado"),
        RepositoryError::AlreadyExists => (StatusCode::CONFLICT, "El identificador ya existe"),
        RepositoryError::Conflict => (StatusCode::CONFLICT, "Conflicto de escritura en CouchDB"),
        RepositoryError::ConnectionError(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "No se pudo conectar con CouchDB",
        ),
        RepositoryError::Unexpected(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error interno al procesar la solicitud",
        ),
    };
    (s, Json(json!({"error":m})))
}

fn validar_devolucion(
    op: &Operacion,
    origin: &Operacion,
    previous: &[Operacion],
) -> Result<(), String> {
    let required_origin = match op.tipo_devolucion {
        Some(TipoDevolucion::Cliente) => TipoOperacion::Venta,
        Some(TipoDevolucion::Proveedor) => TipoOperacion::Compra,
        None => return Err("Falta tipo_devolucion".into()),
    };
    if origin.tipo_operacion != required_origin || origin.puesto_id != op.puesto_id {
        return Err("La operación de origen no corresponde al puesto y tipo de devolución".into());
    }
    let unit_amount = |line: &RenglonOperacion| -> Result<i64, String> {
        let value = if op.tipo_devolucion == Some(TipoDevolucion::Proveedor) {
            line.costo_unitario
        } else {
            line.precio_unitario
        }
        .ok_or("El origen o la devolución no conserva el importe unitario")?;
        parse_scaled(&value.to_string(), 4, 10_000_000_000)
    };
    let mut allowed: HashMap<(String, i64), i64> = HashMap::new();
    for line in &origin.renglones {
        let quantity = parse_scaled(&line.cantidad.to_string(), 3, 1_000_000_000)?;
        *allowed
            .entry((line.producto_id.clone(), unit_amount(line)?))
            .or_default() += quantity;
    }
    for prior in previous.iter().filter(|other| {
        other.tipo_operacion == TipoOperacion::Devolucion
            && other.operacion_origen_id == op.operacion_origen_id
    }) {
        for line in &prior.renglones {
            let quantity = parse_scaled(&line.cantidad.to_string(), 3, 1_000_000_000)?;
            *allowed
                .entry((line.producto_id.clone(), unit_amount(line)?))
                .or_default() -= quantity;
        }
    }
    for line in &op.renglones {
        let quantity = parse_scaled(&line.cantidad.to_string(), 3, 1_000_000_000)?;
        let remaining = allowed
            .get_mut(&(line.producto_id.clone(), unit_amount(line)?))
            .ok_or_else(|| {
                format!(
                    "El producto {} y su importe unitario no figuran en la operación de origen",
                    line.producto_id
                )
            })?;
        *remaining -= quantity;
        if *remaining < 0 {
            return Err(format!(
                "La devolución supera lo pendiente para {}",
                line.producto_id
            ));
        }
    }
    Ok(())
}
pub(crate) fn convertir_autorizado(
    auth: &AuthUser,
    r: OperacionRequest,
) -> Result<Operacion, (StatusCode, Json<Value>)> {
    auth.exigir_propietario(&r.usuario_id)?;
    auth.exigir_puesto(&r.puesto_id)?;
    convertir(r).map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error":e}))))
}

pub async fn crear(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(r): Json<OperacionRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let op = convertir_autorizado(&auth, r)?;
    procesar(&s, op, false).await
}

pub(crate) async fn procesar(
    s: &AppState,
    mut op: Operacion,
    permitir_stock_negativo: bool,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Serialize stock validation and ledger writes within this API process.
    let _guard = s.operation_lock.lock().await;
    let existed = s
        .operaciones
        .obtener_por_id(&op.id)
        .await
        .map_err(map_error)?;
    if existed
        .as_ref()
        .is_some_and(|old| !old.misma_solicitud(&op))
    {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error":"El ID ya corresponde a otra operación"})),
        ));
    }
    if let Some(old) = existed.as_ref() {
        op = old.clone();
    } else {
        op.recibido_en_epoch_ms = Some(crate::domain::date::ahora_epoch_millis());
        for line in &mut op.renglones {
            if let Some(producto) = s
                .productos
                .obtener_por_id(&line.producto_id)
                .await
                .map_err(map_error)?
            {
                line.nombre_producto = Some(producto.nombre);
                line.unidad = Some(producto.unidad);
            }
        }
    }
    if existed.is_none() {
        let puesto = s
            .puestos
            .obtener_por_id(&op.puesto_id)
            .await
            .map_err(map_error)?;
        if puesto.as_ref().is_none_or(|p| !p.activo) {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"error":"El puesto no existe o está inactivo"})),
            ));
        }
        let usuario = s
            .usuarios
            .obtener_por_id(&op.usuario_id)
            .await
            .map_err(map_error)?;
        if usuario.as_ref().is_none_or(|u| !u.activo) {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(
                    json!({"error":"El usuario no existe, está inactivo o no pertenece al puesto"}),
                ),
            ));
        }
        if matches!(op.tipo_operacion, TipoOperacion::Devolucion) {
            let origin_id = op.operacion_origen_id.as_ref().ok_or_else(|| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"Falta operacion_origen_id"})),
                )
            })?;
            let origin = s
                .operaciones
                .obtener_por_id(origin_id)
                .await
                .map_err(map_error)?;
            let origin = origin.ok_or_else(|| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"No existe la operación de origen"})),
                )
            })?;
            let previous = s
                .operaciones
                .listar_por_puesto(&op.puesto_id)
                .await
                .map_err(map_error)?;
            validar_devolucion(&op, &origin, &previous).map_err(|message| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":message})),
                )
            })?;
        }
        if matches!(op.tipo_operacion, TipoOperacion::Ajuste)
            && let Some(conteo_id) = &op.conteo_id
        {
            let count = s
                .conteos
                .obtener_por_id(conteo_id)
                .await
                .map_err(map_error)?
                .ok_or_else(|| {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(json!({"error":"El conteo vinculado no existe"})),
                    )
                })?;
            if count.puesto_id != op.puesto_id || count.origen_registro.as_deref() != Some("pos") {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"El conteo no corresponde a un corte POS del puesto"})),
                ));
            }
            let prior = s
                .operaciones
                .listar_por_puesto(&op.puesto_id)
                .await
                .map_err(map_error)?;
            if prior
                .iter()
                .any(|saved| saved.conteo_id.as_deref() == Some(conteo_id.as_str()))
            {
                return Err((
                    StatusCode::CONFLICT,
                    Json(json!({"error":"Este conteo ya fue ajustado"})),
                ));
            }
            let mut expected: HashMap<String, i64> = HashMap::new();
            for row in &count.renglones {
                let delta =
                    parse_scaled(&row.diferencia.to_string(), 3, i64::MAX).map_err(|message| {
                        (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            Json(json!({"error":message})),
                        )
                    })?;
                if delta != 0 {
                    expected.insert(row.producto_id.clone(), delta);
                }
            }
            let mut actual: HashMap<String, i64> = HashMap::new();
            for row in &op.renglones {
                let delta =
                    parse_scaled(&row.cantidad.to_string(), 3, i64::MAX).map_err(|message| {
                        (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            Json(json!({"error":message})),
                        )
                    })?;
                let prior = actual.get(&row.producto_id).copied().unwrap_or_default();
                let total = prior.checked_add(delta).ok_or_else(|| {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(json!({"error":"Ajuste fuera de rango"})),
                    )
                })?;
                actual.insert(row.producto_id.clone(), total);
            }
            if actual != expected {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error":"El ajuste no coincide con las diferencias del conteo"})),
                ));
            }
        }
    }
    let status = if existed.is_some() {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    CrearOperacion::new(
        s.operaciones.clone(),
        s.movimientos.clone(),
        s.productos.clone(),
    )
    .permitir_stock_negativo(permitir_stock_negativo)
    .ejecutar(op.clone())
    .await
    .map_err(|e| match e {
        CrearOperacionError::DatosInvalidos(m) => {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error": m})))
        }
        CrearOperacionError::StockInsuficiente(m) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": m, "codigo": "stock_insuficiente"})),
        ),
        CrearOperacionError::Repositorio(r) => map_error(r),
    })?;
    Ok((status, Json(respuesta(op))))
}
pub async fn obtener(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match ObtenerOperacion::new(s.operaciones.clone())
        .ejecutar(&id)
        .await
        .map_err(map_error)?
    {
        Some(o) => {
            auth.exigir_puesto(&o.puesto_id)?;
            Ok(Json(respuesta(o)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Operación no encontrada"})),
        )),
    }
}
pub async fn listar(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(f): Query<FiltroPuesto>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    auth.exigir_puesto(&f.puesto_id)?;
    let ops = ListarOperacionesPorPuesto::new(s.operaciones.clone())
        .ejecutar(&f.puesto_id)
        .await
        .map_err(map_error)?;
    Ok(Json(ops.into_iter().map(respuesta).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(total: &str) -> OperacionRequest {
        serde_json::from_str(&format!(r#"{{"id":"operacion:prueba","tipo_operacion":"venta","puesto_id":"puesto:1","usuario_id":"usuario:1","dispositivo_id":"disp","fecha_local":"2026-11-01T10:00:00-06:00","renglones":[{{"producto_id":"producto:1","cantidad":2,"precio_unitario":1.25}}],"total":{total}}}"#)).unwrap()
    }

    #[test]
    fn rechaza_total_distinto_sin_float() {
        assert!(convertir(request("100")).is_err());
        let accepted = convertir(request("2.50")).unwrap();
        assert_eq!(accepted.total_centavos, Some(250));
    }

    #[test]
    fn devolucion_no_puede_cambiar_el_precio_del_origen() {
        let original = convertir(request("2.50")).unwrap();
        let mut returned = original.clone();
        returned.id = "operacion:devolucion".into();
        returned.tipo_operacion = TipoOperacion::Devolucion;
        returned.tipo_devolucion = Some(TipoDevolucion::Cliente);
        returned.operacion_origen_id = Some(original.id.clone());
        returned.renglones[0].cantidad = 1.0;
        returned.renglones[0].precio_unitario = Some(1.30);
        assert!(validar_devolucion(&returned, &original, &[]).is_err());
        returned.renglones[0].precio_unitario = Some(1.25);
        assert!(validar_devolucion(&returned, &original, &[]).is_ok());
    }

    fn pago(estado: &str) -> PagoRequest {
        PagoRequest {
            metodo: "tarjeta".into(),
            modo: "simulado".into(),
            estado: estado.into(),
            autorizacion: "AUT-123456".into(),
            fecha: "2026-11-01T10:00:00-06:00".into(),
        }
    }

    #[test]
    fn acepta_pago_simulado_aprobado() {
        let mut req = request("2.50");
        req.pago = Some(pago("aprobado"));
        let accepted = convertir(req).unwrap();
        assert_eq!(
            accepted.pago.map(|p| p.autorizacion),
            Some("AUT-123456".into())
        );
    }

    #[test]
    fn rechaza_pago_con_estado_o_modo_invalido() {
        let mut req = request("2.50");
        req.pago = Some(pago("desconocido"));
        assert!(convertir(req).is_err());

        let mut req = request("2.50");
        let mut invalido = pago("aprobado");
        invalido.modo = "real".into();
        req.pago = Some(invalido);
        assert!(convertir(req).is_err());
    }
}
