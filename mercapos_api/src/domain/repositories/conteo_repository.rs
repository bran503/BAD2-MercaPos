use async_trait::async_trait;

use crate::domain::entities::conteo_fisico::ConteoFisico;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait ConteoRepository: Send + Sync {
    async fn crear(&self, conteo: ConteoFisico) -> Result<(), RepositoryError>;

    async fn obtener_por_id(&self, id: &str) -> Result<Option<ConteoFisico>, RepositoryError>;

    async fn listar_por_puesto(
        &self,
        puesto_id: &str,
    ) -> Result<Vec<ConteoFisico>, RepositoryError>;
}
