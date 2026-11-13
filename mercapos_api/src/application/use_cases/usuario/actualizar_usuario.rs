use std::sync::Arc;

use crate::application::use_cases::usuario::usuario_error::UsuarioError;
use crate::domain::entities::usuario::Usuario;
use crate::domain::repositories::usuario_repository::UsuarioRepository;

pub type ActualizarUsuarioError = UsuarioError;

pub struct ActualizarUsuario {
    repositorio: Arc<dyn UsuarioRepository>,
}

impl ActualizarUsuario {
    pub fn new(repositorio: Arc<dyn UsuarioRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, usuario: Usuario) -> Result<(), ActualizarUsuarioError> {
        if usuario.id.trim().is_empty() {
            return Err(ActualizarUsuarioError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if usuario.nombre.trim().is_empty() {
            return Err(ActualizarUsuarioError::DatosInvalidos(
                "El nombre es obligatorio".to_string(),
            ));
        }
        if usuario.puesto_id.trim().is_empty() {
            return Err(ActualizarUsuarioError::DatosInvalidos(
                "El puesto es obligatorio".to_string(),
            ));
        }

        self.repositorio.actualizar(usuario).await?;
        Ok(())
    }
}
