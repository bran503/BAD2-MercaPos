#[derive(Debug, Clone, PartialEq)]
pub struct Credencial {
    pub usuario_id: String,
    pub password_hash: String,
    pub actualizado_en: i64,
}
