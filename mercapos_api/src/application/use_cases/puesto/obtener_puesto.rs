use std::sync::Arc;

use crate::domain::entities::puesto::Puesto;
use crate::domain::repositories::puesto_repository::PuestoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ObtenerPuesto {
    repositorio: Arc<dyn PuestoRepository>,
}

impl ObtenerPuesto {
    pub fn new(repositorio: Arc<dyn PuestoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, id: &str) -> Result<Option<Puesto>, RepositoryError> {
        if id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El ID del puesto es obligatorio".to_string(),
            ));
        }
        self.repositorio.obtener_por_id(id).await
    }
}
