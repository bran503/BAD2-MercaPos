use crate::presentation::auth::{AuthUser, respuesta_error};
use crate::{
    app_state::AppState,
    application::{
        auth::{ahora_epoch, hash_password_async, validar_password},
        use_cases::{
            puesto::{
                actualizar_puesto::ActualizarPuesto, crear_puesto::CrearPuesto,
                listar_puestos::ListarPuestos, obtener_puesto::ObtenerPuesto,
            },
            usuario::{
                actualizar_usuario::ActualizarUsuario, crear_usuario::CrearUsuario,
                listar_usuarios_por_puesto::ListarUsuariosPorPuesto,
                obtener_usuario::ObtenerUsuario,
            },
        },
    },
    domain::{
        entities::{
            puesto::Puesto,
            usuario::{Rol, Usuario},
        },
        repositories::repository_error::RepositoryError,
    },
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
pub struct PuestoInput {
    pub id: Option<String>,
    pub revision: Option<String>,
    pub nombre: String,
    pub mercado: String,
    pub ubicacion: String,
    pub propietario_usuario_id: String,
    pub activo: Option<bool>,
}
#[derive(Deserialize)]
pub struct UsuarioInput {
    pub id: Option<String>,
    pub revision: Option<String>,
    pub nombre: String,
    pub rol: String,
    pub puesto_id: String,
    pub activo: Option<bool>,
    #[serde(default)]
    pub password: Option<String>,
}
#[derive(Deserialize)]
pub struct Activo {
    pub activo: Option<bool>,
}
fn error(e: RepositoryError) -> (StatusCode, Json<Value>) {
    let (s, m) = match e {
        RepositoryError::NotFound => (StatusCode::NOT_FOUND, "No encontrado"),
        RepositoryError::AlreadyExists => (StatusCode::CONFLICT, "El identificador ya existe"),
        RepositoryError::Conflict => (StatusCode::CONFLICT, "Conflicto de escritura"),
        RepositoryError::ConnectionError(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "No se pudo conectar con CouchDB",
        ),
        RepositoryError::Unexpected(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Error interno"),
    };
    (s, Json(json!({"error":m})))
}
fn puesto_value(p: Puesto) -> Value {
    json!({"id":p.id,"tipo":"puesto","revision":p.revision,"nombre":p.nombre,"mercado":p.mercado,"ubicacion":p.ubicacion,"propietario_usuario_id":p.propietario_usuario_id,"activo":p.activo})
}
pub(crate) fn usuario_value(u: Usuario) -> Value {
    json!({"id":u.id,"tipo":"usuario","revision":u.revision,"nombre":u.nombre,"rol":match u.rol{Rol::Vendedor=>"vendedor",Rol::Administrador=>"administrador"},"puesto_id":u.puesto_id,"activo":u.activo})
}
pub async fn crear_puesto(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(i): Json<PuestoInput>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.exigir_administrador()?;
    let id = i.id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"id es obligatorio"})),
        )
    })?;
    let p = Puesto {
        id,
        nombre: i.nombre,
        mercado: i.mercado,
        ubicacion: i.ubicacion,
        propietario_usuario_id: i.propietario_usuario_id,
        activo: i.activo.unwrap_or(true),
        revision: None,
    };
    CrearPuesto::new(s.puestos.clone()).ejecutar(p.clone()).await.map_err(|e|match e{crate::application::use_cases::puesto::crear_puesto::CrearPuestoError::DatosInvalidos(m)=>(StatusCode::BAD_REQUEST,Json(json!({"error":m}))),crate::application::use_cases::puesto::crear_puesto::CrearPuestoError::Repositorio(e)=>error(e)})?;
    let saved = s
        .puestos
        .obtener_por_id(&p.id)
        .await
        .map_err(error)?
        .ok_or_else(|| error(RepositoryError::NotFound))?;
    Ok((StatusCode::CREATED, Json(puesto_value(saved))))
}
pub async fn actualizar_puesto(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(i): Json<PuestoInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.exigir_administrador()?;
    auth.exigir_puesto(&id)?;
    if i.id
        .as_deref()
        .is_some_and(|body_id| body_id != id.as_str())
    {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error":"ID del cuerpo no coincide con la ruta"})),
        ));
    }
    let p = Puesto {
        id,
        nombre: i.nombre,
        mercado: i.mercado,
        ubicacion: i.ubicacion,
        propietario_usuario_id: i.propietario_usuario_id,
        activo: i.activo.unwrap_or(true),
        revision: i.revision,
    };
    ActualizarPuesto::new(s.puestos.clone()).ejecutar(p.clone()).await.map_err(|e|match e{crate::application::use_cases::puesto::actualizar_puesto::ActualizarPuestoError::DatosInvalidos(m)=>(StatusCode::BAD_REQUEST,Json(json!({"error":m}))),crate::application::use_cases::puesto::actualizar_puesto::ActualizarPuestoError::Repositorio(e)=>error(e)})?;
    let saved = s
        .puestos
        .obtener_por_id(&p.id)
        .await
        .map_err(error)?
        .ok_or_else(|| error(RepositoryError::NotFound))?;
    Ok(Json(puesto_value(saved)))
}
pub async fn obtener_puesto(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.exigir_puesto(&id)?;
    match ObtenerPuesto::new(s.puestos.clone())
        .ejecutar(&id)
        .await
        .map_err(error)?
    {
        Some(p) => Ok(Json(puesto_value(p))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Puesto no encontrado"})),
        )),
    }
}
pub async fn listar_puestos(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<Activo>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    let mut ps = ListarPuestos::new(s.puestos.clone())
        .ejecutar(q.activo)
        .await
        .map_err(error)?;
    ps.retain(|puesto| puesto.id == auth.puesto_id);
    Ok(Json(ps.into_iter().map(puesto_value).collect()))
}
fn rol(r: &str) -> Result<Rol, String> {
    match r {
        "vendedor" => Ok(Rol::Vendedor),
        "administrador" => Ok(Rol::Administrador),
        _ => Err("rol debe ser vendedor o administrador".into()),
    }
}
pub async fn crear_usuario(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(i): Json<UsuarioInput>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.exigir_administrador()?;
    auth.exigir_puesto(&i.puesto_id)?;
    let password_hash = match &i.password {
        Some(password) => {
            validar_password(password)
                .map_err(|mensaje| respuesta_error(StatusCode::UNPROCESSABLE_ENTITY, mensaje))?;
            Some(
                hash_password_async(password.clone())
                    .await
                    .map_err(|mensaje| {
                        respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, mensaje)
                    })?,
            )
        }
        None => None,
    };
    let id = i.id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"id es obligatorio"})),
        )
    })?;
    let u = Usuario {
        id,
        nombre: i.nombre,
        rol: rol(&i.rol).map_err(|m| (StatusCode::BAD_REQUEST, Json(json!({"error":m}))))?,
        puesto_id: i.puesto_id,
        activo: i.activo.unwrap_or(true),
        revision: None,
    };
    CrearUsuario::new(s.usuarios.clone()).ejecutar(u.clone()).await.map_err(|e|match e{crate::application::use_cases::usuario::crear_usuario::CrearUsuarioError::DatosInvalidos(m)=>(StatusCode::BAD_REQUEST,Json(json!({"error":m}))),crate::application::use_cases::usuario::crear_usuario::CrearUsuarioError::Repositorio(e)=>error(e)})?;
    if let Some(password_hash) = password_hash {
        s.auth
            .guardar_credencial(crate::domain::entities::credencial::Credencial {
                usuario_id: u.id.clone(),
                password_hash,
                actualizado_en: ahora_epoch(),
            })
            .await
            .map_err(error)?;
    }
    let saved = s
        .usuarios
        .obtener_por_id(&u.id)
        .await
        .map_err(error)?
        .ok_or_else(|| error(RepositoryError::NotFound))?;
    Ok((StatusCode::CREATED, Json(usuario_value(saved))))
}
pub async fn actualizar_usuario(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(i): Json<UsuarioInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.exigir_administrador()?;
    let password_hash = match &i.password {
        Some(password) => {
            validar_password(password)
                .map_err(|mensaje| respuesta_error(StatusCode::UNPROCESSABLE_ENTITY, mensaje))?;
            Some(
                hash_password_async(password.clone())
                    .await
                    .map_err(|mensaje| {
                        respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, mensaje)
                    })?,
            )
        }
        None => None,
    };
    if i.id
        .as_deref()
        .is_some_and(|body_id| body_id != id.as_str())
    {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error":"ID del cuerpo no coincide con la ruta"})),
        ));
    }
    let u = Usuario {
        id,
        nombre: i.nombre,
        rol: rol(&i.rol).map_err(|m| (StatusCode::BAD_REQUEST, Json(json!({"error":m}))))?,
        puesto_id: i.puesto_id,
        activo: i.activo.unwrap_or(true),
        revision: i.revision,
    };
    auth.exigir_puesto(&u.puesto_id)?;
    ActualizarUsuario::new(s.usuarios.clone()).ejecutar(u.clone()).await.map_err(|e|match e{crate::application::use_cases::usuario::actualizar_usuario::ActualizarUsuarioError::DatosInvalidos(m)=>(StatusCode::BAD_REQUEST,Json(json!({"error":m}))),crate::application::use_cases::usuario::actualizar_usuario::ActualizarUsuarioError::Repositorio(e)=>error(e)})?;
    if let Some(password_hash) = password_hash {
        s.auth
            .guardar_credencial(crate::domain::entities::credencial::Credencial {
                usuario_id: u.id.clone(),
                password_hash,
                actualizado_en: ahora_epoch(),
            })
            .await
            .map_err(error)?;
    }
    let saved = s
        .usuarios
        .obtener_por_id(&u.id)
        .await
        .map_err(error)?
        .ok_or_else(|| error(RepositoryError::NotFound))?;
    Ok(Json(usuario_value(saved)))
}
pub async fn obtener_usuario(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match ObtenerUsuario::new(s.usuarios.clone())
        .ejecutar(&id)
        .await
        .map_err(error)?
    {
        Some(u) => {
            auth.exigir_puesto(&u.puesto_id)?;
            Ok(Json(usuario_value(u)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Usuario no encontrado"})),
        )),
    }
}
pub async fn listar_usuarios(
    State(s): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(puesto_id): Path<String>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    auth.exigir_puesto(&puesto_id)?;
    let us = ListarUsuariosPorPuesto::new(s.usuarios.clone())
        .ejecutar(&puesto_id)
        .await
        .map_err(error)?;
    Ok(Json(us.into_iter().map(usuario_value).collect()))
}
