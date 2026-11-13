use crate::presentation::auth::AuthUser;
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::domain::entities::producto::Producto;
use serde::Deserialize;
use serde_json::json;

use crate::app_state::AppState;

use crate::application::use_cases::producto::{
    actualizar_producto::ActualizarProducto, crear_producto::CrearProducto,
    listar_productos::ListarProductos, obtener_producto::ObtenerProducto,
};
use crate::presentation::dto::producto_dto::{
    ActualizarProductoRequest, CrearProductoRequest, ProductoResponse,
};
#[derive(Debug, Deserialize)]
pub struct FiltroProducto {
    pub puesto_id: String,
    pub activo: Option<bool>,
}

// POST /productos
pub async fn crear(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CrearProductoRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let producto: Producto = dto.into();
    auth.exigir_administrador()?;
    auth.exigir_propietario(&producto.actualizado_por)?;
    auth.exigir_puesto(&producto.puesto_id)?;
    let puesto = state
        .puestos
        .obtener_por_id(&producto.puesto_id)
        .await
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error":"No se pudo consultar el puesto"})),
            )
        })?;
    if puesto.is_none_or(|p| !p.activo) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"Puesto o administrador inválido"})),
        ));
    }

    let caso_uso = CrearProducto::new(state.productos.clone());

    match caso_uso.ejecutar(producto.clone()).await {
        Ok(()) => {
            let saved = state
                .productos
                .obtener_por_id(&producto.id)
                .await
                .map_err(|_| {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({"error":"No se pudo leer el producto creado"})),
                    )
                })?;
            let respuesta = ProductoResponse::from(saved.unwrap_or(producto));

            Ok((
                StatusCode::CREATED,
                Json(json!({
                    "mensaje": "Producto creado correctamente",
                    "producto": respuesta
                })),
            ))
        }

        Err(error) => {
            let (status, mensaje) = match error {
                crate::application::use_cases::producto::crear_producto::CrearProductoError::DatosInvalidos(mensaje) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, mensaje)
                }

                crate::application::use_cases::producto::crear_producto::CrearProductoError::Repositorio(
                    crate::domain::repositories::repository_error::RepositoryError::AlreadyExists
                ) => {
                    (StatusCode::CONFLICT, "El producto ya existe".to_string())
                }

                crate::application::use_cases::producto::crear_producto::CrearProductoError::Repositorio(error) => {
                    tracing::error!(error=?error, "Could not save product");
                    (if matches!(error, crate::domain::repositories::repository_error::RepositoryError::ConnectionError(_)) { StatusCode::SERVICE_UNAVAILABLE } else { StatusCode::INTERNAL_SERVER_ERROR }, "Error al guardar el producto".to_string())
                }
            };

            Err((
                status,
                Json(json!({
                    "error": mensaje
                })),
            ))
        }
    }
}

// GET /productos
pub async fn listar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(filtro): Query<FiltroProducto>,
) -> Result<Json<Vec<ProductoResponse>>, (StatusCode, Json<serde_json::Value>)> {
    auth.exigir_puesto(&filtro.puesto_id)?;
    let caso_uso = ListarProductos::new(state.productos.clone());

    match caso_uso.ejecutar(&filtro.puesto_id, filtro.activo).await {
        Ok(productos) => {
            let respuesta = productos.into_iter().map(ProductoResponse::from).collect();

            Ok(Json(respuesta))
        }

        Err(error) => {
            tracing::error!(error=?error, "Could not list products");
            let status = if matches!(
                error,
                crate::domain::repositories::repository_error::RepositoryError::ConnectionError(_)
            ) {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((status, Json(json!({"error":"Error al listar productos"}))))
        }
    }
}

pub async fn obtener(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ProductoResponse>, (StatusCode, Json<serde_json::Value>)> {
    let caso_uso = ObtenerProducto::new(state.productos.clone());

    match caso_uso.ejecutar(&id).await {
        Ok(Some(producto)) => {
            auth.exigir_puesto(&producto.puesto_id)?;
            Ok(Json(ProductoResponse::from(producto)))
        }

        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Producto no encontrado"
            })),
        )),

        Err(error) => {
            tracing::error!(error=?error, "Could not get product");

            let status = if matches!(
                error,
                crate::domain::repositories::repository_error::RepositoryError::ConnectionError(_)
            ) {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((
                status,
                Json(json!({
                    "error": "Error al consultar el producto"
                })),
            ))
        }
    }
}

// PUT /productos/{id}
pub async fn actualizar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(dto): Json<ActualizarProductoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.exigir_administrador()?;
    auth.exigir_propietario(&dto.actualizado_por)?;
    auth.exigir_puesto(&dto.puesto_id)?;
    let _guard = state.operation_lock.lock().await;
    let producto = Producto {
        id,
        puesto_id: dto.puesto_id,
        nombre: dto.nombre,
        categoria: dto.categoria,
        precio_unitario: dto.precio_unitario,
        unidad: dto.unidad,
        activo: dto.activo,
        actualizado_en: dto.actualizado_en,
        actualizado_por: dto.actualizado_por,
        revision: Some(dto.revision),
    };
    let current = state
        .productos
        .obtener_por_id(&producto.id)
        .await
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error":"No se pudo consultar el producto"})),
            )
        })?;
    if let Some(current) = current {
        if current.puesto_id != producto.puesto_id {
            return Err((
                StatusCode::CONFLICT,
                Json(json!({"error":"El producto pertenece a otro puesto"})),
            ));
        }
        if current.unidad != producto.unidad {
            let movements = state
                .movimientos
                .listar_por_producto(&producto.id)
                .await
                .map_err(|_| {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({"error":"No se pudo consultar el historial"})),
                    )
                })?;
            if !movements.is_empty() {
                return Err((
                    StatusCode::CONFLICT,
                    Json(
                        json!({"error":"No se puede cambiar la unidad de un producto con historial"}),
                    ),
                ));
            }
        }
    }

    let caso_uso = ActualizarProducto::new(state.productos.clone());

    match caso_uso.ejecutar(producto.clone()).await {
        Ok(()) => {
            let saved = state
                .productos
                .obtener_por_id(&producto.id)
                .await
                .map_err(|_| {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({"error":"No se pudo leer el producto actualizado"})),
                    )
                })?;
            let respuesta = ProductoResponse::from(saved.unwrap_or(producto));

            Ok(Json(json!({
                "mensaje": "Producto actualizado correctamente",
                "producto": respuesta
            })))
        }

        Err(error) => {
            use crate::application::use_cases::producto::actualizar_producto::ActualizarProductoError;
            use crate::domain::repositories::repository_error::RepositoryError;

            let (status, mensaje) = match error {
                ActualizarProductoError::DatosInvalidos(mensaje) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, mensaje)
                }

                ActualizarProductoError::Repositorio(RepositoryError::NotFound) => {
                    (StatusCode::NOT_FOUND, "Producto no encontrado".to_string())
                }

                ActualizarProductoError::Repositorio(RepositoryError::Conflict) => (
                    StatusCode::CONFLICT,
                    "Conflicto al actualizar el producto".to_string(),
                ),

                ActualizarProductoError::Repositorio(RepositoryError::ConnectionError(_)) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "No se pudo conectar con CouchDB".to_string(),
                ),

                ActualizarProductoError::Repositorio(error) => {
                    tracing::error!(error=?error, "Could not update product");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Error al actualizar el producto".to_string(),
                    )
                }
            };

            Err((
                status,
                Json(json!({
                    "error": mensaje
                })),
            ))
        }
    }
}
