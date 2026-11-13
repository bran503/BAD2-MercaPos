const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname, '..', 'design_docs', 'validacion_v2.js'), 'utf8');
const validate = eval(`(${source})`);
const writer = {name: 'mercapos_api', roles: ['mercapos_writer']};
const admin = {name: 'admin', roles: ['_admin']};
const date = '2026-11-01T12:00:00-06:00';
const product = {
  _id: 'producto:test_1', tipo: 'producto', schema_version: 2,
  puesto_id: 'puesto:test_1', nombre: 'Tomate', categoria: 'verdura',
  unidad: 'libra', precio_unitario: 1.25, activo: true,
  actualizado_en: date, actualizado_por: 'usuario:test_admin'
};
const operation = {
  _id: 'operacion:test_1', tipo: 'operacion', schema_version: 2,
  tipo_operacion: 'venta', puesto_id: 'puesto:test_1', usuario_id: 'usuario:test_1',
  dispositivo_id: 'disp_test', fecha_local: date,
  renglones: [{producto_id: product._id, cantidad: 1, precio_unitario: 1.25}],
  impactos_inventario: [{renglon_id: 'r1', producto_id: product._id, cantidad: -1, cantidad_mil: -1000}]
};
const incidence = {
  _id: 'incidencia:operacion:test_1', tipo: 'incidencia', schema_version: 1,
  operacion_id: operation._id, puesto_id: operation.puesto_id,
  usuario_id: operation.usuario_id, tipo_incidencia: 'advertencia', codigo: 422,
  motivo: 'stock insuficiente', intentos: 1, creada_en: 1793556000,
  actualizada_en: 1793556000, resuelta: false
};

function accept(doc, oldDoc = null, user = writer) {
  validate(doc, oldDoc, user, {admins: {names: ['admin'], roles: []}});
}

assert.doesNotThrow(() => accept(product));
assert.doesNotThrow(() => accept(operation));
assert.doesNotThrow(() => accept({...operation, recibido_en_epoch_ms: 1793556000000,
  renglones: [{...operation.renglones[0], nombre_producto: 'Tomate', unidad: 'libra'}]}));
assert.throws(() => accept({...operation, recibido_en_epoch_ms: -1}));
assert.doesNotThrow(() => accept(incidence));
assert.doesNotThrow(() => accept({...incidence, resuelta: true, nota: 'revisada'}, incidence));
assert.doesNotThrow(() => accept({
  _id: 'mov:apertura:1', tipo: 'apertura_inventario', schema_version: 2,
  puesto_id: 'puesto:test_1', producto_id: product._id, usuario_id: null,
  cantidad: 1, cantidad_mil: 1000, fecha_local: date
}));
assert.doesNotThrow(() => accept({
  _id: 'conteo:test_1', tipo: 'conteo_fisico', schema_version: 1,
  puesto_id: 'puesto:test_1', usuario_id: 'usuario:test_1', dispositivo_id: 'disp_test',
  fecha_local: date, renglones: [{producto_id: product._id, cantidad_sistema: 1, cantidad_contada: 0, diferencia: -1}]
}));
const countV2 = {
  _id: 'conteo:test_v2', tipo: 'conteo_fisico', schema_version: 2,
  puesto_id: 'puesto:test_1', usuario_id: 'usuario:test_1', dispositivo_id: 'disp_test',
  fecha_local: date, periodo_estudio: 'piloto-1', etapa: 'antes', origen_registro: 'pos', corte_seq: '4-g1AAA',
  renglones: [{producto_id: product._id, cantidad_sistema: 1, cantidad_contada: 0, diferencia: -1}]
};
assert.doesNotThrow(() => accept(countV2));
assert.throws(() => accept({...countV2, corte_seq: null}));
assert.throws(() => accept({
  _id: 'mov:test_sale', tipo: 'movimiento', puesto_id: 'puesto:test_1', producto_id: product._id,
  cantidad: 1, motivo: 'venta', dispositivo_id: 'disp_test', fecha_local: date
}));

assert.throws(() => accept({...product, puesto_id: undefined}));
assert.throws(() => accept({...product, _id: 'usuario:mal_tipo'}));
assert.throws(() => accept(operation, operation));
assert.throws(() => accept({...incidence, tipo: 'producto'}));
assert.throws(() => accept(product, null, {name: 'intruso', roles: []}));
assert.doesNotThrow(() => accept({...product, _id: '_design/validacion_v2'}, null, admin));

console.log('Validador CouchDB: documentos válidos aceptados e inválidos rechazados.');
