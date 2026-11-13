#[derive(Debug, Clone, PartialEq)]
pub struct ConteoFisico {
    pub id: String,
    pub puesto_id: String,
    pub usuario_id: String,
    pub dispositivo_id: String,
    pub fecha_local: String,
    pub periodo_estudio: Option<String>,
    pub etapa: Option<String>,
    pub origen_registro: Option<String>,
    pub evidencia: Option<String>,
    pub corte_seq: Option<String>,
    pub renglones: Vec<RenglonConteo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenglonConteo {
    pub producto_id: String,
    pub cantidad_sistema: f64,
    pub cantidad_contada: f64,
    pub diferencia: f64,
}
