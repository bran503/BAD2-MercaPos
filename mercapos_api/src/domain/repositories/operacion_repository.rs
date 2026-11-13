use async_trait::async_trait;

use crate::domain::entities::operacion::Operacion;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait OperacionRepository: Send + Sync {
    async fn crear(&self, operacion: Operacion) -> Result<(), RepositoryError>;

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Operacion>, RepositoryError>;

    async fn listar_por_puesto(&self, puesto_id: &str) -> Result<Vec<Operacion>, RepositoryError>;
}
