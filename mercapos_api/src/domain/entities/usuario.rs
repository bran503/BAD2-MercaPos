#[derive(Debug, Clone, PartialEq)]
pub struct Usuario {
    pub id: String,
    pub nombre: String,
    pub rol: Rol,
    pub puesto_id: String,
    pub activo: bool,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rol {
    Vendedor,
    Administrador,
}
