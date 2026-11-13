use async_trait::async_trait;

use crate::domain::entities::usuario::Usuario;
use crate::domain::repositories::repository_error::RepositoryError;

#[async_trait]
pub trait UsuarioRepository: Send + Sync {
    async fn crear(&self, usuario: Usuario) -> Result<(), RepositoryError>;

    async fn actualizar(&self, usuario: Usuario) -> Result<(), RepositoryError>;

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Usuario>, RepositoryError>;

    async fn listar_por_puesto(&self, puesto_id: &str) -> Result<Vec<Usuario>, RepositoryError>;
}
