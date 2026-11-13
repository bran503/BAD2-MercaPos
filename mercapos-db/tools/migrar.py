#!/usr/bin/env python3
"""Audita datos v1 y prepara una base v2 sin modificar el origen.

Modo normal: solo genera snapshot, manifiesto, anomalías y documentos propuestos.
--apply escribe únicamente en una base destino distinta, tras una auditoría limpia.
"""
from __future__ import annotations

import argparse
import base64
from collections import defaultdict
from decimal import Decimal, InvalidOperation, ROUND_HALF_UP
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen


def materialize(value):
    if isinstance(value, Decimal):
        return int(value) if value == value.to_integral_value() else float(value)
    if isinstance(value, dict):
        return {key: materialize(item) for key, item in value.items()}
    if isinstance(value, list):
        return [materialize(item) for item in value]
    return value


def canonical(value):
    return json.dumps(materialize(value), ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def hash_doc(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()


def doc_url_id(document_id):
    """URL-encode a CouchDB document ID while preserving _design's path segment."""
    if document_id.startswith("_design/"):
        return "_design/" + quote(document_id[len("_design/"):], safe="")
    return quote(document_id, safe="")


def decimal(value):
    try:
        result = Decimal(str(value))
    except InvalidOperation as exc:
        raise ValueError(f"cantidad o importe inválido: {value}") from exc
    if not result.is_finite():
        raise ValueError(f"número no finito: {value}")
    return result


def fixture_documents(paths):
    documents = {}
    anomalies = []
    pattern = re.compile(r"(?m)^PUT \{\{host\}\}/mercapos/([^\s?]+)")
    for path in paths:
        source = Path(path).read_text()
        for match in pattern.finditer(source):
            body = re.search(r"\n\s*\n(\{)", source[match.end():])
            if not body:
                continue
            start = match.end() + body.start(1)
            try:
                doc, _ = json.JSONDecoder(parse_float=Decimal).raw_decode(source[start:])
            except json.JSONDecodeError as exc:
                anomalies.append(f"{path}: JSON inválido: {exc}")
                continue
            doc["_id"] = match.group(1)
            old = documents.get(doc["_id"])
            if old is not None and canonical(old) != canonical(doc):
                anomalies.append(f"Documento repetido con contenido diferente: {doc['_id']}")
            documents[doc["_id"]] = doc
    return list(documents.values()), anomalies


class Couch:
    def __init__(self, url, username, password):
        self.url = url.rstrip("/")
        self.username = username
        self.authorization = "Basic " + base64.b64encode(f"{username}:{password}".encode()).decode()

    def request(self, method, path, body=None, allowed=()):
        data = canonical(body).encode() if body is not None else None
        request = Request(self.url + path, data=data, method=method,
                          headers={"Authorization": self.authorization, "Content-Type": "application/json"})
        try:
            with urlopen(request, timeout=30) as response:
                raw = response.read()
                return json.loads(raw, parse_float=Decimal) if raw else {}
        except HTTPError as exc:
            if exc.code in allowed:
                return None
            raise RuntimeError(f"CouchDB respondió {exc.code} en {method} {path}") from exc


def live_documents(couch, database):
    db = quote(database, safe="")
    before = couch.request("GET", f"/{db}")["update_seq"]
    documents = []
    conflicts = []
    last = None
    while True:
        query = {"include_docs": "true", "limit": 500}
        if last is not None:
            query["startkey"] = json.dumps(last)
            query["skip"] = 1
        rows = couch.request("GET", f"/{db}/_all_docs?{urlencode(query)}")["rows"]
        if not rows:
            break
        for row in rows:
            doc = couch.request("GET", f"/{db}/{doc_url_id(row['id'])}?conflicts=true")
            if doc.get("_conflicts"):
                conflicts.append({"id": row["id"], "revisiones": doc["_conflicts"]})
            documents.append(doc)
        last = rows[-1]["id"]
    after = couch.request("GET", f"/{db}")["update_seq"]
    if before != after:
        raise RuntimeError("El origen cambió durante la captura; congele escrituras y repita")
    security = couch.request("GET", f"/{db}/_security")
    indexes = couch.request("GET", f"/{db}/_index")
    return documents, conflicts, before, security, indexes


def clean(doc):
    return {k: v for k, v in doc.items() if k not in {"_rev", "_conflicts", "_revisions", "_revs_info"}}


def expected_delta(operation, line):
    quantity = decimal(line["cantidad"])
    kind = operation["tipo_operacion"]
    if kind in ("venta", "merma"):
        return -abs(quantity)
    if kind == "compra":
        return abs(quantity)
    if kind == "ajuste":
        return quantity
    if kind == "devolucion":
        direction = operation.get("tipo_devolucion")
        if direction == "cliente":
            return abs(quantity)
        if direction == "proveedor":
            return -abs(quantity)
        raise ValueError("Devolución sin tipo_devolucion")
    raise ValueError(f"Tipo de operación desconocido: {kind}")


def audit(documents):
    anomalies = []
    by_id = {doc.get("_id"): doc for doc in documents}
    if len(by_id) != len(documents):
        anomalies.append("Hay IDs duplicados en la captura")
    products = {key: doc for key, doc in by_id.items() if doc.get("tipo") == "producto"}
    operations = {key: doc for key, doc in by_id.items() if doc.get("tipo") == "operacion"}
    movements = [doc for doc in documents if doc.get("tipo") == "movimiento"]
    scopes = defaultdict(set)
    for movement in movements:
        scopes[movement.get("producto_id")].add(movement.get("puesto_id"))
    for operation in operations.values():
        for line in operation.get("renglones", []):
            scopes[line.get("producto_id")].add(operation.get("puesto_id"))
    mapping = {}
    for product_id, product in products.items():
        owners = scopes.get(product_id, set())
        if not owners or None in owners:
            anomalies.append(f"Producto sin ámbito comprobable: {product_id}")
        for owner in owners - {None}:
            mapped = product_id if len(owners) == 1 else f"{product_id}:{owner}"
            mapping[(product_id, owner)] = mapped
    target = []
    for doc in documents:
        if doc.get("tipo") in ("puesto", "usuario"):
            target.append(clean(doc))
        elif doc.get("tipo") == "producto":
            for owner in scopes.get(doc["_id"], set()) - {None}:
                transformed = clean(doc)
                transformed["_id"] = mapping[(doc["_id"], owner)]
                transformed["puesto_id"] = owner
                transformed["schema_version"] = 2
                target.append(transformed)
        elif doc.get("tipo") == "conteo_fisico":
            transformed = clean(doc)
            transformed["schema_version"] = 1
            for line in transformed.get("renglones", []):
                key = (line.get("producto_id"), transformed.get("puesto_id"))
                if key not in mapping:
                    anomalies.append(f"Conteo {doc['_id']} con producto sin ámbito: {key}")
                else:
                    line["producto_id"] = mapping[key]
            target.append(transformed)
    movements_by_operation = defaultdict(list)
    for movement in movements:
        if movement.get("operacion_id"):
            movements_by_operation[movement["operacion_id"]].append(movement)
    for operation_id, operation in operations.items():
        if operation.get("schema_version") == 2:
            target.append(clean(operation))
            continue
        matched = movements_by_operation.pop(operation_id, []).copy()
        effects = []
        for index, line in enumerate(operation.get("renglones", []), 1):
            try:
                signed = expected_delta(operation, line)
            except (KeyError, ValueError) as exc:
                anomalies.append(f"{operation_id}: {exc}")
                continue
            candidates = [m for m in matched if m.get("producto_id") == line.get("producto_id")
                          and decimal(m.get("cantidad", 0)) == signed]
            if len(candidates) != 1:
                anomalies.append(f"{operation_id} r{index}: se esperó 1 movimiento, encontrados {len(candidates)}")
                continue
            matched.remove(candidates[0])
            key = (line["producto_id"], operation.get("puesto_id"))
            if key not in mapping:
                anomalies.append(f"{operation_id}: producto sin ámbito {key}")
                continue
            mil = signed * 1000
            if mil != mil.to_integral_value():
                anomalies.append(f"{operation_id} r{index}: cantidad con más de tres decimales")
            effects.append({"renglon_id": f"r{index}", "producto_id": mapping[key],
                            "cantidad": signed, "cantidad_mil": int(mil)})
        if matched:
            anomalies.append(f"{operation_id}: {len(matched)} movimientos adicionales o duplicados")
        transformed = clean(operation)
        transformed["schema_version"] = 2
        transformed["moneda"] = "USD"
        transformed["impactos_inventario"] = effects
        for line in transformed.get("renglones", []):
            key = (line.get("producto_id"), operation.get("puesto_id"))
            if key in mapping:
                line["producto_id"] = mapping[key]
        if operation.get("total") is not None:
            total = decimal(operation["total"])
            cents = total * 100
            if cents != cents.to_integral_value():
                anomalies.append(f"{operation_id}: total con fracción de centavo")
            else:
                transformed["total_centavos"] = int(cents)
            expected = Decimal(0)
            for line in operation.get("renglones", []):
                unit = line.get("costo_unitario") if operation.get("tipo_operacion") == "compra" else line.get("precio_unitario")
                if unit is None:
                    anomalies.append(f"{operation_id}: falta precio o costo unitario")
                    continue
                expected += (decimal(line["cantidad"]) * decimal(unit)).quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)
            if expected != total:
                anomalies.append(f"{operation_id}: total {total} difiere de renglones {expected}")
        if operation.get("tipo_operacion") == "devolucion":
            source = operations.get(operation.get("operacion_origen_id"))
            expected = "venta" if operation.get("tipo_devolucion") == "cliente" else "compra"
            if not source or source.get("tipo_operacion") != expected or source.get("puesto_id") != operation.get("puesto_id"):
                anomalies.append(f"{operation_id}: referencia de devolución inválida")
        target.append(transformed)
    for operation_id, rows in movements_by_operation.items():
        anomalies.append(f"Movimientos huérfanos de {operation_id}: {len(rows)}")
    opened = set()
    for movement in movements:
        if movement.get("operacion_id"):
            continue
        if movement.get("motivo") != "inventario_inicial":
            anomalies.append(f"Movimiento huérfano no inicial: {movement['_id']}")
            continue
        key = (movement.get("producto_id"), movement.get("puesto_id"))
        if key in opened:
            anomalies.append(f"Apertura repetida para {key}")
        opened.add(key)
        if key not in mapping:
            anomalies.append(f"Apertura sin producto o puesto: {movement['_id']}")
            continue
        transformed = clean(movement)
        transformed["tipo"] = "apertura_inventario"
        transformed["schema_version"] = 2
        transformed["producto_id"] = mapping[key]
        mil = decimal(movement["cantidad"]) * 1000
        if mil != mil.to_integral_value():
            anomalies.append(f"{movement['_id']}: cantidad con más de tres decimales")
        transformed["cantidad_mil"] = int(mil)
        transformed["usuario_id"] = None  # identidad histórica desconocida
        target.append(transformed)
    original_balance = defaultdict(Decimal)
    for movement in movements:
        original_balance[(movement.get("puesto_id"), movement.get("producto_id"))] += decimal(movement["cantidad"])
    new_balance = defaultdict(Decimal)
    for doc in target:
        if doc.get("tipo") == "apertura_inventario":
            new_balance[(doc["puesto_id"], doc["producto_id"])] += decimal(doc["cantidad"])
        elif doc.get("tipo") == "operacion" and doc.get("schema_version") == 2:
            for effect in doc.get("impactos_inventario", []):
                new_balance[(doc["puesto_id"], effect["producto_id"])] += decimal(effect["cantidad"])
    for (owner, old_product), value in original_balance.items():
        mapped = mapping.get((old_product, owner))
        if mapped and value != new_balance[(owner, mapped)]:
            anomalies.append(f"Saldo diferente para {owner}/{old_product}: {value} vs {new_balance[(owner, mapped)]}")
    target.extend(design_documents())
    return target, anomalies, mapping, original_balance, new_balance


def design_documents():
    directory = Path(__file__).resolve().parents[1] / "design_docs"
    inventory = json.loads((directory / "inventario_views.json").read_text())
    validator = (directory / "validacion_v2.js").read_text().strip()
    return [
        {"_id": "_design/inventario", **inventory},
        {"_id": "_design/validacion_v2", "validate_doc_update": validator},
    ]


def index_definitions():
    directory = Path(__file__).resolve().parents[1] / "design_docs"
    return json.loads((directory / "indices.json").read_text())


def write_destination(couch, database, documents):
    db = quote(database, safe="")
    couch.request("PUT", f"/{db}", allowed=(412,))
    for doc in documents:
        path = f"/{db}/{doc_url_id(doc['_id'])}"
        existing = couch.request("GET", path, allowed=(404,))
        if existing is not None:
            if canonical(clean(existing)) != canonical(doc):
                raise RuntimeError(f"Destino ya contiene otro contenido: {doc['_id']}")
            continue
        couch.request("PUT", path, doc)
    couch.request("PUT", f"/{db}/_security", {"admins": {"names": [couch.username], "roles": []},
                                                "members": {"names": [couch.username], "roles": ["mercapos_writer"]}})
    for definition in index_definitions():
        couch.request("POST", f"/{db}/_index", {
            "index": {"fields": definition["fields"]},
            "name": definition["name"],
            "ddoc": definition["ddoc"],
            "type": "json",
        })


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures", nargs="+", type=Path)
    parser.add_argument("--source-url")
    parser.add_argument("--source-db")
    parser.add_argument("--dest-url")
    parser.add_argument("--dest-db")
    parser.add_argument("--report-dir", type=Path, required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    if bool(args.fixtures) == bool(args.source_url):
        parser.error("Indique --fixtures o --source-url junto con --source-db")
    if args.apply and (not args.dest_url or not args.dest_db or args.dest_db == args.source_db):
        parser.error("--apply exige destino explícito y distinto del origen")
    if args.fixtures:
        source, preliminary = fixture_documents(args.fixtures)
        conflicts, sequence = [], None
        security, indexes = None, None
    else:
        password = os.environ.get("COUCHDB_PASSWORD")
        if not password or not args.source_db:
            parser.error("COUCHDB_PASSWORD y --source-db son obligatorios")
        source, conflicts, sequence, security, indexes = live_documents(Couch(args.source_url, os.environ.get("COUCHDB_USERNAME", "admin"), password), args.source_db)
        preliminary = []
    proposed, anomalies, mapping, before, after = audit(source)
    anomalies = preliminary + anomalies + [f"Conflicto en {item['id']}" for item in conflicts]
    os.umask(0o077)
    args.report_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    if stat.S_IMODE(args.report_dir.stat().st_mode) & 0o077:
        raise RuntimeError("El directorio de reporte debe ser privado (chmod 700)")
    (args.report_dir / "origen.json").write_text(json.dumps(materialize(source), ensure_ascii=False, indent=2))
    (args.report_dir / "propuesto.json").write_text(json.dumps(materialize(proposed), ensure_ascii=False, indent=2))
    if security is not None:
        (args.report_dir / "seguridad_origen.json").write_text(json.dumps(materialize(security), ensure_ascii=False, indent=2))
    if indexes is not None:
        (args.report_dir / "indices_origen.json").write_text(json.dumps(materialize(indexes), ensure_ascii=False, indent=2))
    report = {"modo": "aplicación" if args.apply else "dry-run", "origen": args.source_db or "fixtures",
              "destino": args.dest_db, "update_seq": sequence, "documentos_origen": len(source),
              "documentos_propuestos": len(proposed), "anomalías": anomalies,
              "conflictos": conflicts,
              "manifiesto": {doc["_id"]: hash_doc(doc) for doc in source},
              "mapeo_productos": [{"anterior": old, "puesto": owner, "nuevo": new} for (old, owner), new in sorted(mapping.items())],
              "saldos_antes": {f"{k[0]}/{k[1]}": str(v) for k, v in sorted(before.items())},
              "saldos_despues": {f"{k[0]}/{k[1]}": str(v) for k, v in sorted(after.items())}}
    (args.report_dir / "reporte.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
    if anomalies:
        print(f"Auditoría bloqueada: {len(anomalies)} anomalías. Revise {args.report_dir / 'reporte.json'}")
        return 2
    if args.apply:
        password = os.environ.get("COUCHDB_PASSWORD")
        if not password:
            parser.error("COUCHDB_PASSWORD es obligatorio para escribir destino")
        write_destination(Couch(args.dest_url, os.environ.get("COUCHDB_USERNAME", "admin"), password), args.dest_db, proposed)
    print(f"Auditoría limpia; {len(proposed)} documentos propuestos. Reporte: {args.report_dir / 'reporte.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
