use crate::{
    app_state::AppState,
    application::auth::{ahora_epoch, hash_token},
    domain::entities::usuario::Rol,
    domain::repositories::repository_error::RepositoryError,
};
use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub nombre: String,
    pub rol: Rol,
    pub puesto_id: String,
    pub sesion_hash: String,
}

impl AuthUser {
    pub fn es_administrador(&self) -> bool {
        self.rol == Rol::Administrador
    }

    pub fn puede_acceder_puesto(&self, puesto_id: &str) -> bool {
        // Los administradores comerciales también se limitan a su propio puesto.
        self.puesto_id == puesto_id
    }

    pub fn exigir_administrador(&self) -> Result<(), (StatusCode, Json<Value>)> {
        if self.es_administrador() {
            Ok(())
        } else {
            Err(respuesta_error(
                StatusCode::FORBIDDEN,
                "Se requiere rol administrador",
            ))
        }
    }

    pub fn exigir_puesto(&self, puesto_id: &str) -> Result<(), (StatusCode, Json<Value>)> {
        if self.puede_acceder_puesto(puesto_id) {
            Ok(())
        } else {
            Err(respuesta_error(
                StatusCode::FORBIDDEN,
                "No tiene acceso a ese puesto",
            ))
        }
    }

    pub fn exigir_propietario(&self, usuario_id: &str) -> Result<(), (StatusCode, Json<Value>)> {
        if self.id == usuario_id {
            Ok(())
        } else {
            Err(respuesta_error(
                StatusCode::FORBIDDEN,
                "No puede operar en nombre de otro usuario",
            ))
        }
    }
}

pub fn respuesta_error(
    status: StatusCode,
    mensaje: impl Into<String>,
) -> (StatusCode, Json<Value>) {
    (status, Json(json!({"error": mensaje.into()})))
}

pub fn error_repositorio(error: RepositoryError) -> (StatusCode, Json<Value>) {
    match error {
        RepositoryError::NotFound => respuesta_error(StatusCode::NOT_FOUND, "No encontrado"),
        RepositoryError::AlreadyExists => {
            respuesta_error(StatusCode::CONFLICT, "El identificador ya existe")
        }
        RepositoryError::Conflict => {
            respuesta_error(StatusCode::CONFLICT, "Conflicto de escritura")
        }
        RepositoryError::ConnectionError(_) => respuesta_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "No se pudo conectar con CouchDB",
        ),
        RepositoryError::Unexpected(_) => {
            respuesta_error(StatusCode::INTERNAL_SERVER_ERROR, "Error interno")
        }
    }
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|valor| valor.to_str().ok())
        .and_then(|valor| valor.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|valor| !valor.is_empty())
        .map(str::to_owned);
    let Some(token) = token else {
        return Err(respuesta_error(
            StatusCode::UNAUTHORIZED,
            "Falta el token de sesión",
        ));
    };
    let sesion_hash = hash_token(&token);
    let sesion = match state.auth.obtener_sesion(&sesion_hash).await {
        Ok(Some(sesion)) => sesion,
        Ok(None) => {
            return Err(respuesta_error(StatusCode::UNAUTHORIZED, "Sesión inválida"));
        }
        Err(error) => {
            tracing::error!(error=?error, "No se pudo leer la sesión");
            return Err(error_repositorio(error));
        }
    };
    if !sesion.vigente(ahora_epoch()) {
        return Err(respuesta_error(
            StatusCode::UNAUTHORIZED,
            "La sesión expiró o fue revocada",
        ));
    }
    let usuario = match state.usuarios.obtener_por_id(&sesion.usuario_id).await {
        Ok(Some(usuario)) => usuario,
        Ok(None) => {
            return Err(respuesta_error(
                StatusCode::UNAUTHORIZED,
                "Usuario inexistente",
            ));
        }
        Err(error) => {
            tracing::error!(error=?error, "No se pudo leer el usuario de la sesión");
            return Err(error_repositorio(error));
        }
    };
    if !usuario.activo {
        return Err(respuesta_error(
            StatusCode::FORBIDDEN,
            "El usuario está inactivo",
        ));
    }
    request.extensions_mut().insert(AuthUser {
        id: usuario.id,
        nombre: usuario.nombre,
        rol: usuario.rol,
        puesto_id: usuario.puesto_id,
        sesion_hash,
    });
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn administrador_comercial_solo_accede_a_su_puesto() {
        let usuario = AuthUser {
            id: "usuario:admin".into(),
            nombre: "Admin".into(),
            rol: Rol::Administrador,
            puesto_id: "puesto:a".into(),
            sesion_hash: "hash".into(),
        };
        assert!(usuario.puede_acceder_puesto("puesto:a"));
        assert!(!usuario.puede_acceder_puesto("puesto:b"));
    }
}
