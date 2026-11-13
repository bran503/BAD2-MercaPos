use std::sync::Arc;

use crate::domain::entities::usuario::Usuario;
use crate::domain::repositories::repository_error::RepositoryError;
use crate::domain::repositories::usuario_repository::UsuarioRepository;

pub struct ListarUsuariosPorPuesto {
    repositorio: Arc<dyn UsuarioRepository>,
}

impl ListarUsuariosPorPuesto {
    pub fn new(repositorio: Arc<dyn UsuarioRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, puesto_id: &str) -> Result<Vec<Usuario>, RepositoryError> {
        if puesto_id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El puesto_id es obligatorio".to_string(),
            ));
        }
        self.repositorio.listar_por_puesto(puesto_id).await
    }
}
