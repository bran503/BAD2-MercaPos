use std::sync::Arc;

use crate::domain::entities::puesto::Puesto;
use crate::domain::repositories::puesto_repository::PuestoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum CrearPuestoError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for CrearPuestoError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct CrearPuesto {
    repositorio: Arc<dyn PuestoRepository>,
}

impl CrearPuesto {
    pub fn new(repositorio: Arc<dyn PuestoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, puesto: Puesto) -> Result<(), CrearPuestoError> {
        if puesto.id.trim().is_empty() {
            return Err(CrearPuestoError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if puesto.nombre.trim().is_empty() {
            return Err(CrearPuestoError::DatosInvalidos(
                "El nombre es obligatorio".to_string(),
            ));
        }
        if puesto.mercado.trim().is_empty() {
            return Err(CrearPuestoError::DatosInvalidos(
                "El mercado es obligatorio".to_string(),
            ));
        }
        if puesto.ubicacion.trim().is_empty() {
            return Err(CrearPuestoError::DatosInvalidos(
                "La ubicación es obligatoria".to_string(),
            ));
        }
        if puesto.propietario_usuario_id.trim().is_empty() {
            return Err(CrearPuestoError::DatosInvalidos(
                "El propietario es obligatorio".to_string(),
            ));
        }

        self.repositorio.crear(puesto).await?;
        Ok(())
    }
}
