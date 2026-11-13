use std::sync::Arc;

use crate::domain::date::valid_rfc3339;
use crate::domain::entities::producto::Producto;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum CrearProductoError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for CrearProductoError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct CrearProducto {
    repositorio: Arc<dyn ProductoRepository>,
}

impl CrearProducto {
    pub fn new(repositorio: Arc<dyn ProductoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, producto: Producto) -> Result<(), CrearProductoError> {
        // Validar identificador
        if producto.id.trim().is_empty() {
            return Err(CrearProductoError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if !producto.id.starts_with("producto:") || !producto.puesto_id.starts_with("puesto:") {
            return Err(CrearProductoError::DatosInvalidos(
                "ID o puesto_id inválido".into(),
            ));
        }

        // Validar nombre
        if producto.nombre.trim().is_empty() {
            return Err(CrearProductoError::DatosInvalidos(
                "El nombre es obligatorio".to_string(),
            ));
        }

        // Validar categoría
        if producto.categoria.trim().is_empty() {
            return Err(CrearProductoError::DatosInvalidos(
                "La categoría es obligatoria".to_string(),
            ));
        }

        // Validar precio
        if !producto.precio_unitario.is_finite() || producto.precio_unitario < 0.0 {
            return Err(CrearProductoError::DatosInvalidos(
                "El precio debe ser un número válido y no negativo".to_string(),
            ));
        }

        // Validar unidad
        if producto.unidad.trim().is_empty() {
            return Err(CrearProductoError::DatosInvalidos(
                "La unidad es obligatoria".to_string(),
            ));
        }
        if !valid_rfc3339(&producto.actualizado_en) {
            return Err(CrearProductoError::DatosInvalidos(
                "actualizado_en debe ser RFC 3339 con zona".into(),
            ));
        }

        // Persistir el producto
        self.repositorio.crear(producto).await?;

        Ok(())
    }
}
