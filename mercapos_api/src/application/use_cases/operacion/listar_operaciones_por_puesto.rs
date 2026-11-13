use std::sync::Arc;

use crate::domain::entities::operacion::Operacion;
use crate::domain::repositories::operacion_repository::OperacionRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ListarOperacionesPorPuesto {
    repositorio: Arc<dyn OperacionRepository>,
}

impl ListarOperacionesPorPuesto {
    pub fn new(repositorio: Arc<dyn OperacionRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, puesto_id: &str) -> Result<Vec<Operacion>, RepositoryError> {
        if puesto_id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El puesto_id es obligatorio".to_string(),
            ));
        }
        self.repositorio.listar_por_puesto(puesto_id).await
    }
}
