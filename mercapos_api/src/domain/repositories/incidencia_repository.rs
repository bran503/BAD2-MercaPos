use async_trait::async_trait;

use crate::domain::entities::incidencia::IncidenciaSincronizacion;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait IncidenciaRepository: Send + Sync {
    async fn guardar(&self, incidencia: IncidenciaSincronizacion) -> Result<(), RepositoryError>;

    async fn obtener_por_operacion(
        &self,
        operacion_id: &str,
    ) -> Result<Option<IncidenciaSincronizacion>, RepositoryError>;

    async fn listar(
        &self,
        puesto_id: Option<&str>,
        resuelta: Option<bool>,
    ) -> Result<Vec<IncidenciaSincronizacion>, RepositoryError>;

    async fn resolver(
        &self,
        operacion_id: &str,
        resuelta_por: &str,
        nota: Option<String>,
        ahora: i64,
    ) -> Result<Option<IncidenciaSincronizacion>, RepositoryError>;
}
