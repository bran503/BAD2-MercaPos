use async_trait::async_trait;

use crate::domain::entities::producto::Producto;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait ProductoRepository: Send + Sync {
    async fn crear(&self, producto: Producto) -> Result<(), RepositoryError>;

    async fn actualizar(&self, producto: Producto) -> Result<(), RepositoryError>;

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Producto>, RepositoryError>;

    async fn listar(
        &self,
        puesto_id: &str,
        activo: Option<bool>,
    ) -> Result<Vec<Producto>, RepositoryError>;
}
