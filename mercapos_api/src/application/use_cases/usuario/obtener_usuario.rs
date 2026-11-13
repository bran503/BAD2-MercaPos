use std::sync::Arc;

use crate::domain::entities::usuario::Usuario;
use crate::domain::repositories::repository_error::RepositoryError;
use crate::domain::repositories::usuario_repository::UsuarioRepository;

pub struct ObtenerUsuario {
    repositorio: Arc<dyn UsuarioRepository>,
}

impl ObtenerUsuario {
    pub fn new(repositorio: Arc<dyn UsuarioRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, id: &str) -> Result<Option<Usuario>, RepositoryError> {
        if id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El ID del usuario es obligatorio".to_string(),
            ));
        }
        self.repositorio.obtener_por_id(id).await
    }
}
