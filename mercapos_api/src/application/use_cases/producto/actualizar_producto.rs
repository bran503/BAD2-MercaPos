use std::sync::Arc;

use crate::domain::date::valid_rfc3339;
use crate::domain::entities::producto::Producto;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum ActualizarProductoError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for ActualizarProductoError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct ActualizarProducto {
    repositorio: Arc<dyn ProductoRepository>,
}

impl ActualizarProducto {
    pub fn new(repositorio: Arc<dyn ProductoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, producto: Producto) -> Result<(), ActualizarProductoError> {
        if producto.id.trim().is_empty() {
            return Err(ActualizarProductoError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if !producto.id.starts_with("producto:") || !producto.puesto_id.starts_with("puesto:") {
            return Err(ActualizarProductoError::DatosInvalidos(
                "ID o puesto_id inválido".into(),
            ));
        }

        if producto.nombre.trim().is_empty() {
            return Err(ActualizarProductoError::DatosInvalidos(
                "El nombre es obligatorio".to_string(),
            ));
        }

        if producto.categoria.trim().is_empty() {
            return Err(ActualizarProductoError::DatosInvalidos(
                "La categoría es obligatoria".to_string(),
            ));
        }

        if !producto.precio_unitario.is_finite() || producto.precio_unitario < 0.0 {
            return Err(ActualizarProductoError::DatosInvalidos(
                "El precio debe ser válido y no negativo".to_string(),
            ));
        }

        if producto.unidad.trim().is_empty() {
            return Err(ActualizarProductoError::DatosInvalidos(
                "La unidad es obligatoria".to_string(),
            ));
        }
        if !valid_rfc3339(&producto.actualizado_en) {
            return Err(ActualizarProductoError::DatosInvalidos(
                "actualizado_en debe ser RFC 3339 con zona".into(),
            ));
        }

        self.repositorio.actualizar(producto).await?;

        Ok(())
    }
}
