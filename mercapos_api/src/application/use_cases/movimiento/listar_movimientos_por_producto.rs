use std::sync::Arc;

use crate::domain::entities::movimiento::Movimiento;
use crate::domain::repositories::movimiento_repository::MovimientoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

pub struct ListarMovimientosPorProducto {
    repositorio: Arc<dyn MovimientoRepository>,
}

impl ListarMovimientosPorProducto {
    pub fn new(repositorio: Arc<dyn MovimientoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, producto_id: &str) -> Result<Vec<Movimiento>, RepositoryError> {
        if producto_id.trim().is_empty() {
            return Err(RepositoryError::Unexpected(
                "El producto_id es obligatorio".to_string(),
            ));
        }
        self.repositorio.listar_por_producto(producto_id).await
    }
}
