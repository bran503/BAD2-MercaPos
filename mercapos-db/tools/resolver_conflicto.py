#!/usr/bin/env python3
"""Resolve every current CouchDB leaf after an explicit catalog choice.

Dry-run is the default. Financial operation conflicts are deliberately refused;
they require investigation and a compensating business event, not branch deletion.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from urllib.parse import quote

from migrar import Couch

TIPOS_RESOLUBLES = {"producto", "puesto", "usuario"}
MAX_REINTENTOS_LECTURA = 3


def revision_leaves(document: dict) -> set[str]:
    current = document.get("_rev")
    if not isinstance(current, str):
        raise RuntimeError("Documento sin _rev vigente")
    conflicts = document.get("_conflicts", [])
    if not isinstance(conflicts, list) or not all(isinstance(x, str) for x in conflicts):
        raise RuntimeError("_conflicts tiene formato inválido")
    return {current, *conflicts}


def doc_path(database: str, document_id: str) -> str:
    return f"/{quote(database, safe='')}/{quote(document_id, safe='')}"


def get_conflicted(couch: Couch, path: str) -> dict:
    document = couch.request("GET", path + "?conflicts=true")
    if document.get("_deleted"):
        raise RuntimeError("El documento está eliminado; no se resuelve automáticamente")
    kind = document.get("tipo")
    if kind not in TIPOS_RESOLUBLES:
        if kind == "operacion":
            raise RuntimeError("Conflicto financiero: investigue y conserve evidencia; no se borran ramas")
        raise RuntimeError(f"Tipo de documento no resoluble: {kind!r}")
    return document


def selected_branch(couch: Couch, path: str, keep_revision: str, leaves: set[str]) -> dict:
    if keep_revision not in leaves:
        raise RuntimeError("La revisión elegida ya no es una hoja vigente; vuelva a inspeccionar el conflicto")
    doc = couch.request("GET", path + "?rev=" + quote(keep_revision, safe="-"))
    if doc.get("_id") is None:
        raise RuntimeError("La revisión elegida no contiene un documento")
    return doc


def clean_branch(document: dict) -> dict:
    return {
        key: value
        for key, value in document.items()
        if key not in {"_rev", "_conflicts", "_revisions", "_revs_info"}
    }


def resolve(couch: Couch, database: str, document_id: str, keep_revision: str, apply: bool) -> None:
    path = doc_path(database, document_id)
    before = get_conflicted(couch, path)
    leaves = revision_leaves(before)
    if keep_revision not in leaves:
        raise RuntimeError("La revisión elegida no es una hoja vigente")
    branch = selected_branch(couch, path, keep_revision, leaves)
    if branch.get("_id") != document_id or branch.get("tipo") != before.get("tipo"):
        raise RuntimeError("La revisión elegida cambió el ID o el tipo del documento")
    # Al promover la rama elegida al tronco actual, todas las hojas en conflicto
    # deben eliminarse, incluida la elegida si era una rama perdedora original.
    conflicts_before = set(before.get("_conflicts", []))
    print(json.dumps({
        "id": document_id,
        "tipo": before["tipo"],
        "revision_elegida": keep_revision,
        "hojas": sorted(leaves),
        "ramas_a_eliminar": sorted(conflicts_before),
        "aplicar": apply,
    }, ensure_ascii=False, indent=2))
    if not apply:
        print("Simulación solamente. Revise los datos de cada revisión y repita con --apply.")
        return

    selected = clean_branch(branch)
    selected["_id"] = document_id
    selected["_rev"] = before["_rev"]
    try:
        saved = couch.request("PUT", path, selected, allowed=(409,))
    except Exception as error:
        raise RuntimeError(f"No se pudo escribir la revisión elegida: {error}") from error
    if saved is None:
        latest = get_conflicted(couch, path)
        if keep_revision not in revision_leaves(latest):
            raise RuntimeError("El documento cambió y la revisión elegida ya no está disponible; vuelva a elegir")
        raise RuntimeError("Conflicto concurrente al guardar; repita la auditoría con las revisiones actuales")
    main_revision = saved.get("rev")
    if not isinstance(main_revision, str):
        raise RuntimeError("CouchDB no devolvió la nueva revisión principal")

    # CouchDB may receive another replicated branch while leaves are deleted.
    # Only remove revisions observed in the snapshot; preserve unknown new leaves.
    expected = conflicts_before
    for _ in range(MAX_REINTENTOS_LECTURA):
        latest = get_conflicted(couch, path)
        if latest.get("_rev") != main_revision:
            raise RuntimeError("El ganador cambió concurrentemente; se conservaron las hojas restantes")
        current_conflicts = set(latest.get("_conflicts", []))
        unknown = current_conflicts - expected
        if unknown:
            raise RuntimeError(f"Aparecieron hojas nuevas concurrentes: {sorted(unknown)}")
        if not current_conflicts:
            if clean_branch(latest) != clean_branch(selected):
                raise RuntimeError("El contenido principal cambió durante la resolución")
            print("Conflicto resuelto; el contenido elegido quedó como revisión principal.")
            return
        for revision in sorted(current_conflicts):
            delete_path = path + "?rev=" + quote(revision, safe="-")
            couch.request("DELETE", delete_path, allowed=(404, 409))
    raise RuntimeError("Quedaron conflictos tras varios intentos; revise las hojas manualmente")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=os.environ.get("COUCHDB_URL", "http://127.0.0.1:5985"))
    parser.add_argument("--database", required=True)
    parser.add_argument("--id", required=True)
    parser.add_argument("--keep-rev", required=True, help="Revisión elegida tras revisar manualmente todas las hojas")
    parser.add_argument("--apply", action="store_true", help="Escribe el ganador y elimina todas las hojas perdedoras observadas")
    args = parser.parse_args()
    try:
        couch = Couch(
            args.url,
            os.environ["COUCHDB_USERNAME"],
            os.environ["COUCHDB_PASSWORD"],
        )
        resolve(couch, args.database, args.id, args.keep_rev, args.apply)
        return 0
    except KeyError as error:
        print(f"Falta variable de entorno: {error.args[0]}", file=sys.stderr)
    except Exception as error:
        print(f"No se resolvió el conflicto: {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
