use std::sync::Arc;

use crate::domain::entities::puesto::Puesto;
use crate::domain::repositories::puesto_repository::PuestoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ListarPuestos {
    repositorio: Arc<dyn PuestoRepository>,
}

impl ListarPuestos {
    pub fn new(repositorio: Arc<dyn PuestoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, activo: Option<bool>) -> Result<Vec<Puesto>, RepositoryError> {
        self.repositorio.listar(activo).await
    }
}
