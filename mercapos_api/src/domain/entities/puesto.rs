#[derive(Debug, Clone, PartialEq)]
pub struct Puesto {
    pub id: String,
    pub nombre: String,
    pub mercado: String,
    pub ubicacion: String,
    pub propietario_usuario_id: String,
    pub activo: bool,
    pub revision: Option<String>,
}
