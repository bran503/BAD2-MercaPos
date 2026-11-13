function (n, o, u, s) {
  var roles = (u && u.roles) || [];
  var admin = roles.indexOf('_admin') !== -1;
  function reject(message) { throw({forbidden: message}); }
  function text(value) { return typeof value === 'string' && value.trim().length > 0; }
  function number(value) { return typeof value === 'number' && isFinite(value); }
  function integer(value) { return number(value) && Math.floor(value) === value && Math.abs(value) <= 9007199254740991; }
  function prefix(id, expected) { return typeof id === 'string' && id.indexOf(expected) === 0; }
  function date(value) { return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$/.test(value); }

  if (!n || !text(n._id)) reject('ID obligatorio');
  if (n._id.indexOf('_design/') === 0) {
    if (!admin) reject('Diseño reservado al administrador');
    return;
  }
  if (!admin && roles.indexOf('mercapos_writer') === -1) reject('Escritura reservada a la API');
  if (n._deleted) reject('No se permite borrar documentos comerciales');
  if (!text(n.tipo)) reject('tipo obligatorio');

  var prefixes = {
    puesto: 'puesto:', usuario: 'usuario:', producto: 'producto:',
    operacion: 'operacion:', movimiento: 'mov:', apertura_inventario: 'mov:',
    conteo_fisico: 'conteo:', incidencia: 'incidencia:'
  };
  if (!prefixes[n.tipo] || !prefix(n._id, prefixes[n.tipo])) reject('tipo o prefijo inválido');
  if (o && o.tipo !== n.tipo) reject('No se permite cambiar el tipo');
  if (o && ['operacion', 'movimiento', 'apertura_inventario', 'conteo_fisico'].indexOf(o.tipo) !== -1) {
    reject('Evento comercial inmutable');
  }

  if (n.tipo === 'puesto') {
    if (!text(n.nombre) || !text(n.mercado) || !text(n.ubicacion) || !text(n.propietario_usuario_id) || typeof n.activo !== 'boolean') reject('Puesto incompleto');
  } else if (n.tipo === 'usuario') {
    if (!text(n.nombre) || ['vendedor', 'administrador'].indexOf(n.rol) === -1 || !prefix(n.puesto_id, 'puesto:') || typeof n.activo !== 'boolean') reject('Usuario incompleto');
  } else if (n.tipo === 'producto') {
    if (n.schema_version !== 2 || !prefix(n.puesto_id, 'puesto:') || !text(n.nombre) || !text(n.categoria) || !text(n.unidad) || !number(n.precio_unitario) || n.precio_unitario < 0 || typeof n.activo !== 'boolean' || !date(n.actualizado_en) || !prefix(n.actualizado_por, 'usuario:')) reject('Producto v2 incompleto o inválido');
  } else if (n.tipo === 'operacion') {
    if (n.schema_version !== 2 || !prefix(n.puesto_id, 'puesto:') || !prefix(n.usuario_id, 'usuario:') || !text(n.dispositivo_id) || !date(n.fecha_local) || !Array.isArray(n.renglones) || n.renglones.length < 1 || !Array.isArray(n.impactos_inventario)) reject('Operación v2 incompleta');
    if (['venta', 'compra', 'merma', 'devolucion', 'ajuste'].indexOf(n.tipo_operacion) === -1) reject('Tipo de operación inválido');
    if (n.renglones.length > 100 || (n.total !== undefined && (!number(n.total) || n.total < 0)) || (n.total_centavos !== undefined && (!integer(n.total_centavos) || n.total_centavos < 0))) reject('Importe o cantidad de renglones fuera de rango');
    for (var i = 0; i < n.renglones.length; i++) {
      var line = n.renglones[i];
      if (!prefix(line.producto_id, 'producto:') || !number(line.cantidad) || line.cantidad === 0 || (line.precio_unitario !== undefined && (!number(line.precio_unitario) || line.precio_unitario < 0)) || (line.costo_unitario !== undefined && (!number(line.costo_unitario) || line.costo_unitario < 0))) reject('Renglón de operación inválido');
      if ((line.nombre_producto !== undefined && !text(line.nombre_producto)) || (line.unidad !== undefined && !text(line.unidad))) reject('Snapshot de producto inválido');
    }
    if (n.recibido_en_epoch_ms !== undefined && (!integer(n.recibido_en_epoch_ms) || n.recibido_en_epoch_ms < 0)) reject('Instante de recepción inválido');
    if (n.pago !== undefined) {
      var pago = n.pago;
      if (n.tipo_operacion !== 'venta' || !pago || pago.metodo !== 'tarjeta' || pago.modo !== 'simulado' || ['aprobado', 'rechazado', 'cancelado'].indexOf(pago.estado) === -1 || !text(pago.autorizacion) || !date(pago.fecha)) reject('Pago simulado inválido');
    }
    for (var j = 0; j < n.impactos_inventario.length; j++) {
      var impact = n.impactos_inventario[j];
      if (!prefix(impact.producto_id, 'producto:') || !text(impact.renglon_id) || !integer(impact.cantidad_mil) || impact.cantidad_mil === 0) reject('Impacto de inventario inválido');
    }
  } else if (n.tipo === 'movimiento') {
    if (!prefix(n.puesto_id, 'puesto:') || !prefix(n.producto_id, 'producto:') || !number(n.cantidad) || n.cantidad === 0 || ['inventario_inicial', 'venta', 'compra', 'merma', 'devolucion', 'ajuste'].indexOf(n.motivo) === -1 || !text(n.dispositivo_id) || !date(n.fecha_local)) reject('Movimiento inválido');
    if (['inventario_inicial', 'compra'].indexOf(n.motivo) !== -1 && n.cantidad < 0) reject('Entrada de inventario con signo inválido');
    if (['venta', 'merma'].indexOf(n.motivo) !== -1 && n.cantidad > 0) reject('Salida de inventario con signo inválido');
  } else if (n.tipo === 'apertura_inventario') {
    if (n.schema_version !== 2 || !prefix(n.puesto_id, 'puesto:') || !prefix(n.producto_id, 'producto:') || (n.usuario_id !== null && n.usuario_id !== undefined && !prefix(n.usuario_id, 'usuario:')) || !integer(n.cantidad_mil) || n.cantidad_mil <= 0 || !number(n.cantidad) || !date(n.fecha_local)) reject('Apertura de inventario inválida');
  } else if (n.tipo === 'conteo_fisico') {
    if ([1, 2].indexOf(n.schema_version) === -1 || !prefix(n.puesto_id, 'puesto:') || !prefix(n.usuario_id, 'usuario:') || !text(n.dispositivo_id) || !date(n.fecha_local) || !Array.isArray(n.renglones) || n.renglones.length < 1) reject('Conteo incompleto');
    if (n.schema_version === 2 && (!text(n.periodo_estudio) || ['antes', 'despues'].indexOf(n.etapa) === -1 || ['cuaderno', 'pos'].indexOf(n.origen_registro) === -1 || (n.origen_registro === 'cuaderno' && !text(n.evidencia)) || (n.origen_registro === 'pos' && !text(n.corte_seq)))) reject('Trazabilidad del conteo v2 incompleta');
    for (var k = 0; k < n.renglones.length; k++) {
      var row = n.renglones[k];
      if (!prefix(row.producto_id, 'producto:') || !number(row.cantidad_sistema) || !number(row.cantidad_contada) || !number(row.diferencia) || Math.abs((row.cantidad_contada - row.cantidad_sistema) - row.diferencia) > 0.000001) reject('Renglón de conteo inválido');
    }
  } else if (n.tipo === 'incidencia') {
    if (n.schema_version !== 1 || !prefix(n.operacion_id, 'operacion:') || !prefix(n.puesto_id, 'puesto:') || !prefix(n.usuario_id, 'usuario:') || ['rechazo', 'advertencia'].indexOf(n.tipo_incidencia) === -1 || !integer(n.codigo) || n.codigo < 0 || n.codigo > 65535 || !text(n.motivo) || !integer(n.intentos) || n.intentos < 1 || !integer(n.creada_en) || !integer(n.actualizada_en) || typeof n.resuelta !== 'boolean') reject('Incidencia inválida');
  }
}
