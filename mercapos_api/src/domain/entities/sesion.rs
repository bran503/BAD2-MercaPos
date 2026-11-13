#[derive(Debug, Clone, PartialEq)]
pub struct Sesion {
    pub token_hash: String,
    pub usuario_id: String,
    pub creada_en: i64,
    pub expira_en: i64,
    pub revocada: bool,
}

impl Sesion {
    pub fn vigente(&self, ahora: i64) -> bool {
        !self.revocada && self.expira_en > ahora
    }
}
