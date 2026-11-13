use async_trait::async_trait;

use crate::domain::entities::puesto::Puesto;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait PuestoRepository: Send + Sync {
    async fn crear(&self, puesto: Puesto) -> Result<(), RepositoryError>;

    async fn actualizar(&self, puesto: Puesto) -> Result<(), RepositoryError>;

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Puesto>, RepositoryError>;

    async fn listar(&self, activo: Option<bool>) -> Result<Vec<Puesto>, RepositoryError>;
}
