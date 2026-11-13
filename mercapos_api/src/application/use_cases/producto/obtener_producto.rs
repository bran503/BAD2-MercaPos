use std::sync::Arc;

use crate::domain::entities::producto::Producto;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ObtenerProducto {
    repositorio: Arc<dyn ProductoRepository>,
}

impl ObtenerProducto {
    pub fn new(repositorio: Arc<dyn ProductoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, id: &str) -> Result<Option<Producto>, RepositoryError> {
        if id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El ID del producto es obligatorio".to_string(),
            ));
        }

        self.repositorio.obtener_por_id(id).await
    }
}
