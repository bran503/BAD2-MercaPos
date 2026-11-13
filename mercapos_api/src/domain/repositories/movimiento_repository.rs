use async_trait::async_trait;

use crate::domain::entities::movimiento::Movimiento;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait MovimientoRepository: Send + Sync {
    async fn crear(&self, movimiento: Movimiento) -> Result<(), RepositoryError>;

    async fn listar_por_producto(
        &self,
        producto_id: &str,
    ) -> Result<Vec<Movimiento>, RepositoryError>;

    async fn listar_por_puesto(&self, puesto_id: &str) -> Result<Vec<Movimiento>, RepositoryError>;
}
