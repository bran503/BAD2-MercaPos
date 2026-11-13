use serde::{Deserialize, Serialize};

use crate::domain::entities::producto::Producto;

#[derive(Debug, Deserialize)]
pub struct CrearProductoRequest {
    pub id: String,
    pub puesto_id: String,
    pub nombre: String,
    pub categoria: String,
    pub precio_unitario: f64,
    pub unidad: String,
    pub actualizado_en: String,
    pub actualizado_por: String,
}

#[derive(Debug, Deserialize)]
pub struct ActualizarProductoRequest {
    pub revision: String,
    pub puesto_id: String,
    pub nombre: String,
    pub categoria: String,
    pub precio_unitario: f64,
    pub unidad: String,
    pub activo: bool,
    pub actualizado_en: String,
    pub actualizado_por: String,
}

#[derive(Debug, Serialize)]
pub struct ProductoResponse {
    pub revision: Option<String>,
    pub id: String,
    pub puesto_id: String,
    pub nombre: String,
    pub categoria: String,
    pub precio_unitario: f64,
    pub unidad: String,
    pub activo: bool,
    pub actualizado_en: String,
    pub actualizado_por: String,
}

impl From<CrearProductoRequest> for Producto {
    fn from(dto: CrearProductoRequest) -> Self {
        Self {
            id: dto.id,
            puesto_id: dto.puesto_id,
            nombre: dto.nombre,
            categoria: dto.categoria,
            precio_unitario: dto.precio_unitario,
            unidad: dto.unidad,
            activo: true,
            actualizado_en: dto.actualizado_en,
            actualizado_por: dto.actualizado_por,
            revision: None,
        }
    }
}

impl From<Producto> for ProductoResponse {
    fn from(producto: Producto) -> Self {
        Self {
            id: producto.id,
            puesto_id: producto.puesto_id,
            nombre: producto.nombre,
            categoria: producto.categoria,
            precio_unitario: producto.precio_unitario,
            unidad: producto.unidad,
            activo: producto.activo,
            actualizado_en: producto.actualizado_en,
            actualizado_por: producto.actualizado_por,
            revision: producto.revision,
        }
    }
}
