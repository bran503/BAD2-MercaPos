use crate::{
    app_state::AppState,
    application::{
        auth::{
            ahora_epoch, generar_token, hash_password_async, hash_token, validar_password,
            verificar_password_async,
        },
        use_cases::{puesto::crear_puesto::CrearPuesto, usuario::crear_usuario::CrearUsuario},
    },
    domain::entities::{
        puesto::Puesto,
        sesion::Sesion,
        usuario::{Rol, Usuario},
    },
    presentation::{
        auth::{AuthUser, error_repositorio, respuesta_error},
        routes::catalogo::usuario_value,
    },
};
use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub usuario_id: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct CambiarPasswordRequest {
    pub actual: String,
    pub nueva: String,
}

#[derive(Deserialize)]
pub struct BootstrapRequest {
    pub puesto_id: String,
    pub puesto_nombre: String,
    pub mercado: String,
    pub ubicacion: String,
    pub usuario_id: String,
    pub usuario_nombre: String,
    pub password: String,
}

async fn crear_sesion(
    state: &AppState,
    usuario_id: &str,
) -> Result<(String, i64), (StatusCode, Json<Value>)> {
    let token = generar_token();
    let ahora = ahora_epoch();
    let expira = ahora.saturating_add(state.session_ttl_seconds);
    let sesion = Sesion {
        token_hash: hash_token(&token),
        usuario_id: usuario_id.to_owned(),
        creada_en: ahora,
        expira_en: expira,
        revocada: false,
    };
    state
        .auth
        .crear_sesion(sesion)
        .await
        .map_err(error_repositorio)?;
    Ok((token, expira))
}

fn credenciales_invalidas() -> (StatusCode, Json<Value>) {
    respuesta_error(StatusCode::UNAUTHORIZED, "Credenciales inválidas")
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if request.usuario_id.is_empty()
        || request.usuario_id.len() > 128
        || request.password.chars().count() > 128
        || !request
            .usuario_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
    {
        return Err(credenciales_invalidas());
    }
    if !state.login_limiter.permitido(&request.usuario_id) {
        return Err(respuesta_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Demasiados intentos; espere antes de volver a intentar",
        ));
    }
    let credencial = state
        .auth
        .obtener_credencial(&request.usuario_id)
        .await
        .map_err(error_repositorio)?;
    let Some(credencial) = credencial else {
        state.login_limiter.registrar_fallo(&request.usuario_id);
        return Err(credenciales_invalidas());
    };
    if !verificar_password_async(request.password, credencial.password_hash).await {
        state.login_limiter.registrar_fallo(&request.usuario_id);
        return Err(credenciales_invalidas());
    }
    let usuario = state
        .usuarios
        .obtener_por_id(&request.usuario_id)
        .await
        .map_err(error_repositorio)?
        .ok_or_else(credenciales_invalidas)?;
    if !usuario.activo {
        return Err(respuesta_error(
            StatusCode::FORBIDDEN,
            "El usuario está inactivo",
        ));
    }
    state.login_limiter.registrar_exito(&request.usuario_id);
    let (token, expira) = crear_sesion(&state, &usuario.id).await?;
    Ok(Json(json!({
        "token": token,
        "expira_en": expira,
        "usuario": usuario_value(usuario),
    })))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(usuario): Extension<AuthUser>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    state
        .auth
        .revocar_sesion(&usuario.sesion_hash)
        .await
        .map_err(error_repositorio)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(
    State(state): State<AppState>,
    Extension(usuario): Extension<AuthUser>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let actual = state
        .usuarios
        .obtener_por_id(&usuario.id)
        .await
        .map_err(error_repositorio)?
        .ok_or_else(|| respuesta_error(StatusCode::NOT_FOUND, "Usuario no encontrado"))?;
    Ok(Json(json!({"usuario": usuario_value(actual)})))
}

pub async fn cambiar_password(
    State(state): State<AppState>,
    Extension(usuario): Extension<AuthUser>,
    Json(request): Json<CambiarPasswordRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let credencial = state
        .auth
        .obtener_credencial(&usuario.id)
        .await
        .map_err(error_repositorio)?
        .ok_or_else(credenciales_invalidas)?;
    if !verificar_password_async(request.actual, credencial.password_hash).await {
        return Err(credenciales_invalidas());
    }
    validar_password(&request.nueva)
        .map_err(|mensaje| respuesta_error(StatusCode::UNPROCESSABLE_ENTITY, mensaje))?;
    let hash = hash_password_async(request.nueva)
        .await
        .map_err(|mensaje| respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, mensaje))?;
    state
        .auth
        .guardar_credencial(crate::domain::entities::credencial::Credencial {
            usuario_id: usuario.id.clone(),
            password_hash: hash,
            actualizado_en: ahora_epoch(),
        })
        .await
        .map_err(error_repositorio)?;
    Ok(Json(json!({"mensaje": "Contraseña actualizada"})))
}

pub async fn bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BootstrapRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let esperado = state
        .bootstrap_token
        .as_deref()
        .ok_or_else(|| respuesta_error(StatusCode::FORBIDDEN, "Bootstrap deshabilitado"))?;
    let recibido = headers
        .get("x-bootstrap-token")
        .and_then(|valor| valor.to_str().ok())
        .unwrap_or_default();
    if recibido != esperado {
        return Err(respuesta_error(
            StatusCode::FORBIDDEN,
            "Token de bootstrap inválido",
        ));
    }
    // Evita dos bootstraps simultáneos dentro de la instancia escritora soportada.
    let _guard = state.bootstrap_lock.lock().await;
    if state
        .auth
        .contar_credenciales()
        .await
        .map_err(error_repositorio)?
        > 0
    {
        return Err(respuesta_error(
            StatusCode::CONFLICT,
            "Ya existe al menos un usuario con credenciales",
        ));
    }
    validar_password(&request.password)
        .map_err(|mensaje| respuesta_error(StatusCode::UNPROCESSABLE_ENTITY, mensaje))?;
    if request.usuario_id.trim().is_empty() || request.puesto_id.trim().is_empty() {
        return Err(respuesta_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "usuario_id y puesto_id son obligatorios",
        ));
    }
    let puesto_id = request.puesto_id.clone();
    let puesto = Puesto {
        id: request.puesto_id,
        nombre: request.puesto_nombre,
        mercado: request.mercado,
        ubicacion: request.ubicacion,
        propietario_usuario_id: request.usuario_id.clone(),
        activo: true,
        revision: None,
    };
    CrearPuesto::new(state.puestos.clone())
        .ejecutar(puesto)
        .await
        .map_err(|error| {
            match error {
            crate::application::use_cases::puesto::crear_puesto::CrearPuestoError::DatosInvalidos(
                mensaje,
            ) => respuesta_error(StatusCode::UNPROCESSABLE_ENTITY, mensaje),
            crate::application::use_cases::puesto::crear_puesto::CrearPuestoError::Repositorio(
                error,
            ) => error_repositorio(error),
        }
        })?;
    let usuario = Usuario {
        id: request.usuario_id.clone(),
        nombre: request.usuario_nombre,
        rol: Rol::Administrador,
        puesto_id,
        activo: true,
        revision: None,
    };
    CrearUsuario::new(state.usuarios.clone())
        .ejecutar(usuario.clone())
        .await
        .map_err(|error| match error {
            crate::application::use_cases::usuario::usuario_error::UsuarioError::DatosInvalidos(
                mensaje,
            ) => respuesta_error(StatusCode::UNPROCESSABLE_ENTITY, mensaje),
            crate::application::use_cases::usuario::usuario_error::UsuarioError::Repositorio(
                error,
            ) => error_repositorio(error),
        })?;
    let hash = hash_password_async(request.password)
        .await
        .map_err(|mensaje| respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, mensaje))?;
    state
        .auth
        .guardar_credencial(crate::domain::entities::credencial::Credencial {
            usuario_id: usuario.id.clone(),
            password_hash: hash,
            actualizado_en: ahora_epoch(),
        })
        .await
        .map_err(error_repositorio)?;
    let (token, expira) = crear_sesion(&state, &usuario.id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": token,
            "expira_en": expira,
            "usuario": usuario_value(usuario),
        })),
    ))
}
