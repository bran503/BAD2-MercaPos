use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use backend::application::use_cases::operacion::crear_operacion::{
    CrearOperacion, CrearOperacionError,
};
use backend::domain::entities::movimiento::{DireccionMovimiento, MotivoMovimiento, Movimiento};
use backend::domain::entities::operacion::{
    Operacion, RenglonOperacion, TipoDevolucion, TipoOperacion,
};
use backend::domain::entities::producto::Producto;
use backend::domain::repositories::movimiento_repository::MovimientoRepository;
use backend::domain::repositories::operacion_repository::OperacionRepository;
use backend::domain::repositories::producto_repository::ProductoRepository;
use backend::domain::repositories::repository_error::RepositoryError;

struct ProductosFake(HashMap<String, Producto>);

#[async_trait]
impl ProductoRepository for ProductosFake {
    async fn crear(&self, _: Producto) -> Result<(), RepositoryError> {
        unreachable!()
    }
    async fn actualizar(&self, _: Producto) -> Result<(), RepositoryError> {
        unreachable!()
    }
    async fn obtener_por_id(&self, id: &str) -> Result<Option<Producto>, RepositoryError> {
        Ok(self.0.get(id).cloned())
    }
    async fn listar(&self, _: &str, _: Option<bool>) -> Result<Vec<Producto>, RepositoryError> {
        Ok(self.0.values().cloned().collect())
    }
}

#[derive(Default)]
struct MovimientosFake(
    Mutex<Vec<Movimiento>>,
    Arc<OperacionesFake>,
    Mutex<HashSet<String>>,
);

impl MovimientosFake {
    fn projected(&self) -> Vec<Movimiento> {
        let mut rows = self.0.lock().unwrap().clone();
        for operation in self.1.0.lock().unwrap().values() {
            if self.2.lock().unwrap().contains(&operation.id) {
                continue;
            }
            if operation.destino_devolucion.as_deref() == Some("no_vendible") {
                continue;
            }
            for (index, line) in operation.renglones.iter().enumerate() {
                let outgoing = matches!(
                    operation.tipo_operacion,
                    TipoOperacion::Venta | TipoOperacion::Merma
                ) || operation.tipo_devolucion == Some(TipoDevolucion::Proveedor)
                    || (matches!(operation.tipo_operacion, TipoOperacion::Ajuste)
                        && line.cantidad < 0.0);
                let motivo = match operation.tipo_operacion {
                    TipoOperacion::Venta => MotivoMovimiento::Venta,
                    TipoOperacion::Compra => MotivoMovimiento::Compra,
                    TipoOperacion::Merma => MotivoMovimiento::Merma,
                    TipoOperacion::Devolucion => MotivoMovimiento::Devolucion,
                    TipoOperacion::Ajuste => MotivoMovimiento::Ajuste,
                };
                rows.push(Movimiento {
                    id: format!(
                        "mov:{}:r{}:{}",
                        operation.id.trim_start_matches("operacion:"),
                        index + 1,
                        line.producto_id.trim_start_matches("producto:")
                    ),
                    producto_id: line.producto_id.clone(),
                    puesto_id: operation.puesto_id.clone(),
                    cantidad: line.cantidad.abs(),
                    motivo,
                    operacion_id: Some(operation.id.clone()),
                    usuario_id: Some(operation.usuario_id.clone()),
                    dispositivo_id: operation.dispositivo_id.clone(),
                    fecha_local: operation.fecha_local.clone(),
                    direccion: if outgoing {
                        DireccionMovimiento::Salida
                    } else {
                        DireccionMovimiento::Entrada
                    },
                });
            }
        }
        rows
    }
}

#[async_trait]
impl MovimientoRepository for MovimientosFake {
    async fn crear(&self, movimiento: Movimiento) -> Result<(), RepositoryError> {
        let mut movements = self.0.lock().unwrap();
        if movements.iter().any(|saved| saved.id == movimiento.id) {
            return Err(RepositoryError::AlreadyExists);
        }
        movements.push(movimiento);
        Ok(())
    }
    async fn listar_por_producto(&self, id: &str) -> Result<Vec<Movimiento>, RepositoryError> {
        Ok(self
            .projected()
            .into_iter()
            .filter(|m| m.producto_id == id)
            .collect())
    }
    async fn listar_por_puesto(&self, id: &str) -> Result<Vec<Movimiento>, RepositoryError> {
        Ok(self
            .projected()
            .into_iter()
            .filter(|m| m.puesto_id == id)
            .collect())
    }
}

#[derive(Default)]
struct OperacionesFake(Mutex<HashMap<String, Operacion>>);

#[async_trait]
impl OperacionRepository for OperacionesFake {
    async fn crear(&self, operacion: Operacion) -> Result<(), RepositoryError> {
        let mut saved = self.0.lock().unwrap();
        if saved.contains_key(&operacion.id) {
            return Err(RepositoryError::AlreadyExists);
        }
        saved.insert(operacion.id.clone(), operacion);
        Ok(())
    }
    async fn obtener_por_id(&self, id: &str) -> Result<Option<Operacion>, RepositoryError> {
        Ok(self.0.lock().unwrap().get(id).cloned())
    }
    async fn listar_por_puesto(&self, id: &str) -> Result<Vec<Operacion>, RepositoryError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .values()
            .filter(|o| o.puesto_id == id)
            .cloned()
            .collect())
    }
}

fn producto() -> Producto {
    Producto {
        id: "producto:0042".into(),
        puesto_id: "puesto:0001".into(),
        nombre: "Tomate".into(),
        categoria: "verduras".into(),
        precio_unitario: 1.25,
        unidad: "libra".into(),
        activo: true,
        actualizado_en: "2026-10-30T09:00:00-06:00".into(),
        actualizado_por: "usuario:0002".into(),
        revision: None,
    }
}

fn entrada(cantidad: f64) -> Movimiento {
    Movimiento {
        id: "mov:disp03:20261030T080000:r1:0042".into(),
        producto_id: "producto:0042".into(),
        puesto_id: "puesto:0001".into(),
        cantidad,
        motivo: MotivoMovimiento::InventarioInicial,
        operacion_id: None,
        usuario_id: Some("usuario:0002".into()),
        dispositivo_id: "disp03".into(),
        fecha_local: "2026-10-30T08:00:00-06:00".into(),
        direccion: DireccionMovimiento::Entrada,
    }
}

fn venta(cantidad: f64) -> Operacion {
    Operacion {
        id: "operacion:disp03:20261030T090000".into(),
        tipo_operacion: TipoOperacion::Venta,
        puesto_id: "puesto:0001".into(),
        usuario_id: "usuario:0001".into(),
        dispositivo_id: "disp03".into(),
        fecha_local: "2026-10-30T09:00:00-06:00".into(),
        recibido_en_epoch_ms: None,
        renglones: vec![RenglonOperacion {
            producto_id: "producto:0042".into(),
            cantidad,
            precio_unitario: Some(1.25),
            costo_unitario: None,
            nombre_producto: None,
            unidad: None,
        }],
        total: Some(cantidad * 1.25),
        total_centavos: Some((cantidad * 125.0) as i64),
        motivo: None,
        operacion_origen_id: None,
        conteo_id: None,
        tipo_devolucion: None,
        destino_devolucion: None,
        pago: None,
    }
}

fn servicio(stock: f64) -> (CrearOperacion, Arc<MovimientosFake>, Arc<OperacionesFake>) {
    let mut products = HashMap::new();
    products.insert("producto:0042".into(), producto());
    let products = Arc::new(ProductosFake(products));
    let operations = Arc::new(OperacionesFake::default());
    let movements = Arc::new(MovimientosFake(
        Mutex::new(Vec::new()),
        operations.clone(),
        Mutex::new(HashSet::new()),
    ));
    if stock > 0.0 {
        movements.0.lock().unwrap().push(entrada(stock));
    }
    (
        CrearOperacion::new(operations.clone(), movements.clone(), products),
        movements,
        operations,
    )
}

#[tokio::test]
async fn venta_valida_es_un_solo_evento_y_reintento_no_duplica() {
    let (service, movements, operations) = servicio(5.0);
    let operation = venta(2.0);
    service.ejecutar(operation.clone()).await.unwrap();
    service.ejecutar(operation).await.unwrap();
    let saved = movements.0.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(operations.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn venta_rechaza_existencia_insuficiente() {
    let (service, _, _) = servicio(1.0);
    assert!(matches!(
        service.ejecutar(venta(2.0)).await,
        Err(CrearOperacionError::StockInsuficiente(_))
    ));
}

#[tokio::test]
async fn segunda_venta_no_ignora_el_evento_canonico_ya_guardado() {
    let (service, movements, operations) = servicio(5.0);
    service.ejecutar(venta(5.0)).await.unwrap();
    let mut second = venta(1.0);
    second.id = "operacion:segunda".into();
    assert!(matches!(
        service.ejecutar(second).await,
        Err(CrearOperacionError::StockInsuficiente(_))
    ));
    assert_eq!(movements.0.lock().unwrap().len(), 1);
    assert_eq!(operations.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn venta_offline_puede_aceptarse_con_stock_negativo() {
    let (service, movements, operations) = servicio(1.0);
    service
        .permitir_stock_negativo(true)
        .ejecutar(venta(2.0))
        .await
        .unwrap();
    assert_eq!(operations.0.lock().unwrap().len(), 1);
    let projected = movements.projected();
    let neto: f64 = projected
        .iter()
        .map(|movimiento| {
            if matches!(movimiento.direccion, DireccionMovimiento::Salida) {
                -movimiento.cantidad
            } else {
                movimiento.cantidad
            }
        })
        .sum();
    assert_eq!(neto, -1.0);
}

#[tokio::test]
async fn reenvio_legacy_no_crea_otro_movimiento() {
    let (service, movements, operations) = servicio(5.0);
    let original = venta(2.0);
    operations
        .0
        .lock()
        .unwrap()
        .insert(original.id.clone(), original.clone());
    movements.2.lock().unwrap().insert(original.id.clone());
    movements.0.lock().unwrap().push(Movimiento {
        id: "mov:disp03:20261030T090000:r1".into(),
        producto_id: "producto:0042".into(),
        puesto_id: "puesto:0001".into(),
        cantidad: 2.0,
        motivo: MotivoMovimiento::Venta,
        operacion_id: Some(original.id.clone()),
        usuario_id: None,
        dispositivo_id: "disp03".into(),
        fecha_local: original.fecha_local.clone(),
        direccion: DireccionMovimiento::Salida,
    });
    service.ejecutar(original).await.unwrap();
    assert_eq!(movements.0.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn ajuste_negativo_queda_en_operacion_canonica() {
    let (service, movements, operations) = servicio(3.0);
    let mut operation = venta(1.0);
    operation.id = "operacion:disp03:20261030T180500".into();
    operation.tipo_operacion = TipoOperacion::Ajuste;
    operation.renglones[0].cantidad = -1.0;
    operation.renglones[0].precio_unitario = None;
    operation.total = None;
    operation.total_centavos = None;
    operation.motivo = Some("Diferencia de conteo".into());
    service.ejecutar(operation.clone()).await.unwrap();
    let saved = movements.0.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(
        operations.0.lock().unwrap().get(&operation.id),
        Some(&operation)
    );
}
