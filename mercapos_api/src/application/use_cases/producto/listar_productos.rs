use std::sync::Arc;

use crate::domain::entities::producto::Producto;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ListarProductos {
    repositorio: Arc<dyn ProductoRepository>,
}

impl ListarProductos {
    pub fn new(repositorio: Arc<dyn ProductoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(
        &self,
        puesto_id: &str,
        activo: Option<bool>,
    ) -> Result<Vec<Producto>, RepositoryError> {
        self.repositorio.listar(puesto_id, activo).await
    }
}
