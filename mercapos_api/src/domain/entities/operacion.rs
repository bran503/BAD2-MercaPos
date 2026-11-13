#[derive(Debug, Clone, PartialEq)]
pub struct Operacion {
    pub id: String,
    pub tipo_operacion: TipoOperacion,
    pub puesto_id: String,
    pub usuario_id: String,
    pub dispositivo_id: String,
    pub fecha_local: String,
    /// Hora UTC asignada por la API al aceptar la operación (Unix ms).
    pub recibido_en_epoch_ms: Option<u64>,
    pub renglones: Vec<RenglonOperacion>,
    pub total: Option<f64>,
    pub total_centavos: Option<i64>,
    pub motivo: Option<String>,
    pub operacion_origen_id: Option<String>,
    pub conteo_id: Option<String>,
    pub tipo_devolucion: Option<TipoDevolucion>,
    pub destino_devolucion: Option<String>,
    /// Resultado del pago simulado con tarjeta, si la venta lo registró.
    pub pago: Option<Pago>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pago {
    pub metodo: String,
    pub modo: String,
    pub estado: String,
    pub autorizacion: String,
    pub fecha: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TipoOperacion {
    Venta,
    Compra,
    Merma,
    Devolucion,
    Ajuste,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TipoDevolucion {
    Cliente,
    Proveedor,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RenglonOperacion {
    pub producto_id: String,
    pub cantidad: f64,
    pub precio_unitario: Option<f64>,
    pub costo_unitario: Option<f64>,
    /// Valores del catálogo conservados al aceptar el evento.
    pub nombre_producto: Option<String>,
    pub unidad: Option<String>,
}

impl Operacion {
    /// Compara el contenido enviado, ignorando metadatos inmutables generados por servidor.
    pub fn misma_solicitud(&self, otra: &Self) -> bool {
        let mut original = self.clone();
        let mut recibida = otra.clone();
        original.recibido_en_epoch_ms = None;
        recibida.recibido_en_epoch_ms = None;
        for renglon in &mut original.renglones {
            renglon.nombre_producto = None;
            renglon.unidad = None;
        }
        for renglon in &mut recibida.renglones {
            renglon.nombre_producto = None;
            renglon.unidad = None;
        }
        original == recibida
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reintento_ignora_solo_metadatos_generados_por_servidor() {
        let original = Operacion {
            id: "operacion:test".into(),
            tipo_operacion: TipoOperacion::Venta,
            puesto_id: "puesto:1".into(),
            usuario_id: "usuario:1".into(),
            dispositivo_id: "disp1".into(),
            fecha_local: "2026-11-01T10:00:00-06:00".into(),
            recibido_en_epoch_ms: None,
            renglones: vec![RenglonOperacion {
                producto_id: "producto:1".into(),
                cantidad: 2.0,
                precio_unitario: Some(1.25),
                costo_unitario: None,
                nombre_producto: None,
                unidad: None,
            }],
            total: Some(2.5),
            total_centavos: Some(250),
            motivo: None,
            operacion_origen_id: None,
            conteo_id: None,
            tipo_devolucion: None,
            destino_devolucion: None,
            pago: None,
        };
        let mut guardada = original.clone();
        guardada.recibido_en_epoch_ms = Some(1_798_000_000_000);
        guardada.renglones[0].nombre_producto = Some("Tomate".into());
        guardada.renglones[0].unidad = Some("libra".into());
        assert!(guardada.misma_solicitud(&original));
        guardada.renglones[0].cantidad = 3.0;
        assert!(!guardada.misma_solicitud(&original));
    }
}
