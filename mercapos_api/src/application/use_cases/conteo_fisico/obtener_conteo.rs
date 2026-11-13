use std::sync::Arc;

use crate::domain::entities::conteo_fisico::ConteoFisico;
use crate::domain::repositories::conteo_repository::ConteoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ObtenerConteo {
    repositorio: Arc<dyn ConteoRepository>,
}

impl ObtenerConteo {
    pub fn new(repositorio: Arc<dyn ConteoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, id: &str) -> Result<Option<ConteoFisico>, RepositoryError> {
        if id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El ID del conteo es obligatorio".to_string(),
            ));
        }
        self.repositorio.obtener_por_id(id).await
    }
}
