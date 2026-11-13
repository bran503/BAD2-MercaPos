use async_trait::async_trait;

use crate::domain::entities::credencial::Credencial;
use crate::domain::entities::sesion::Sesion;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn guardar_credencial(&self, credencial: Credencial) -> Result<(), RepositoryError>;

    async fn obtener_credencial(
        &self,
        usuario_id: &str,
    ) -> Result<Option<Credencial>, RepositoryError>;

    async fn eliminar_credencial(&self, usuario_id: &str) -> Result<(), RepositoryError>;

    async fn contar_credenciales(&self) -> Result<u64, RepositoryError>;

    async fn crear_sesion(&self, sesion: Sesion) -> Result<(), RepositoryError>;

    async fn obtener_sesion(&self, token_hash: &str) -> Result<Option<Sesion>, RepositoryError>;

    async fn revocar_sesion(&self, token_hash: &str) -> Result<(), RepositoryError>;
}
