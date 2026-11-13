#[derive(Debug, Clone, PartialEq)]
pub struct Producto {
    pub id: String,
    pub puesto_id: String,
    pub nombre: String,
    pub categoria: String,
    pub precio_unitario: f64,
    pub unidad: String,
    pub activo: bool,
    pub actualizado_en: String,
    pub actualizado_por: String,
    pub revision: Option<String>,
}
