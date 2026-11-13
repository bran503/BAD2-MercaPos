use std::sync::Arc;

use crate::domain::entities::conteo_fisico::ConteoFisico;
use crate::domain::repositories::conteo_repository::ConteoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ListarConteosPorPuesto {
    repositorio: Arc<dyn ConteoRepository>,
}

impl ListarConteosPorPuesto {
    pub fn new(repositorio: Arc<dyn ConteoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, puesto_id: &str) -> Result<Vec<ConteoFisico>, RepositoryError> {
        if puesto_id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El puesto_id es obligatorio".to_string(),
            ));
        }
        self.repositorio.listar_por_puesto(puesto_id).await
    }
}
