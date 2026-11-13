#!/usr/bin/env python3
"""Install/update MercaPOS v2 design docs and Mango indexes in an existing DB.

Requires an administrator account. Never creates or changes _security and never
touches business documents. Safe to rerun; design updates use CouchDB _rev.
"""
from __future__ import annotations

import argparse
import os
import sys
from urllib.parse import quote

from migrar import Couch, clean, design_documents, index_definitions, canonical, doc_url_id


def install(couch: Couch, database: str) -> None:
    db = quote(database, safe="")
    couch.request("GET", f"/{db}")  # Require an existing database; do not create one accidentally.

    for template in design_documents():
        path = f"/{db}/{doc_url_id(template['_id'])}"
        existing = couch.request("GET", path, allowed=(404,))
        document = dict(template)
        if existing is not None:
            old = clean(existing)
            document = {**old, **template}
            if isinstance(old.get("views"), dict) and isinstance(template.get("views"), dict):
                document["views"] = {**old["views"], **template["views"]}
            if canonical(old) == canonical(document):
                continue
            revision = existing.get("_rev")
            if not isinstance(revision, str):
                raise RuntimeError(f"El diseño {template['_id']} no tiene _rev; se detuvo sin sobrescribir")
            document["_rev"] = revision
        couch.request("PUT", path, document)

    for definition in index_definitions():
        couch.request("POST", f"/{db}/_index", {
            "index": {"fields": definition["fields"]},
            "name": definition["name"],
            "ddoc": definition["ddoc"],
            "type": "json",
        })


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=os.environ.get("COUCHDB_URL", "http://127.0.0.1:5985"))
    parser.add_argument("--database", required=True, help="Base v2 existente, nueva o ya migrada")
    args = parser.parse_args()
    try:
        couch = Couch(
            args.url,
            os.environ["COUCHDB_ADMIN_USERNAME"],
            os.environ["COUCHDB_ADMIN_PASSWORD"],
        )
        install(couch, args.database)
        print(f"Esquema v2 e índices verificados en {args.database}; _security y documentos de negocio no se modificaron.")
        return 0
    except KeyError as error:
        print(f"Falta variable de entorno: {error.args[0]}", file=sys.stderr)
    except Exception as error:
        print(f"No se pudo instalar el esquema: {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
