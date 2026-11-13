use std::sync::Arc;

use crate::domain::entities::operacion::Operacion;
use crate::domain::repositories::operacion_repository::OperacionRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ObtenerOperacion {
    repositorio: Arc<dyn OperacionRepository>,
}

impl ObtenerOperacion {
    pub fn new(repositorio: Arc<dyn OperacionRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, id: &str) -> Result<Option<Operacion>, RepositoryError> {
        if id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El ID de la operación es obligatorio".to_string(),
            ));
        }
        self.repositorio.obtener_por_id(id).await
    }
}
