use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoIncidencia {
    Rechazo,
    Advertencia,
}

impl TipoIncidencia {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rechazo => "rechazo",
            Self::Advertencia => "advertencia",
        }
    }

    pub fn desde_str(valor: &str) -> Option<Self> {
        match valor {
            "rechazo" => Some(Self::Rechazo),
            "advertencia" => Some(Self::Advertencia),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncidenciaSincronizacion {
    pub id: String,
    pub operacion_id: String,
    pub puesto_id: String,
    pub usuario_id: String,
    pub tipo: TipoIncidencia,
    pub codigo: u16,
    pub motivo: String,
    pub intentos: u32,
    pub creada_en: i64,
    pub actualizada_en: i64,
    pub resuelta: bool,
    pub resuelta_por: Option<String>,
    pub resuelta_en: Option<i64>,
    pub nota: Option<String>,
    pub revision: Option<String>,
}

pub fn id_de_incidencia(operacion_id: &str) -> String {
    let valido = !operacion_id.is_empty()
        && operacion_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'));
    if valido {
        return format!("incidencia:{operacion_id}");
    }
    let digest = Sha256::digest(operacion_id.as_bytes());
    format!("incidencia:{}", hex::encode(&digest[..16]))
}
