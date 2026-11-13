#[derive(Debug, Clone, PartialEq)]
pub struct Movimiento {
    pub id: String,
    pub producto_id: String,
    pub puesto_id: String,
    pub cantidad: f64,
    pub motivo: MotivoMovimiento,
    pub operacion_id: Option<String>,
    pub usuario_id: Option<String>,
    pub dispositivo_id: String,
    pub fecha_local: String,
    pub direccion: DireccionMovimiento,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotivoMovimiento {
    InventarioInicial,
    Venta,
    Compra,
    Merma,
    Devolucion,
    Ajuste,
}
#[derive(Debug, Clone, PartialEq)]
pub enum DireccionMovimiento {
    Entrada,
    Salida,
}
