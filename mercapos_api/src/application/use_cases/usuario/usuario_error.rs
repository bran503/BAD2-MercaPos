use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum UsuarioError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for UsuarioError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}
