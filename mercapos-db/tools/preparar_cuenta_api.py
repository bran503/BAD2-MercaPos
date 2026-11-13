#!/usr/bin/env python3
"""Create the CouchDB data-service account and grant its database role.

All credentials are read from the environment. Re-running is safe: an existing
account must already have the expected role and is never silently reconfigured.
"""
from __future__ import annotations

import base64
import json
import os
import sys
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen


class Couch:
    def __init__(self) -> None:
        self.url = os.environ.get("COUCHDB_URL", "http://127.0.0.1:5985").rstrip("/")
        username = os.environ["COUCHDB_ADMIN_USERNAME"]
        password = os.environ["COUCHDB_ADMIN_PASSWORD"]
        token = base64.b64encode(f"{username}:{password}".encode()).decode()
        self.headers = {"Authorization": f"Basic {token}", "Content-Type": "application/json"}

    def request(self, method: str, path: str, body=None, allow=()):
        data = json.dumps(body).encode() if body is not None else None
        req = Request(self.url + path, data=data, method=method, headers=self.headers)
        try:
            with urlopen(req, timeout=15) as response:
                return json.loads(response.read() or b"{}")
        except HTTPError as error:
            if error.code in allow:
                return None
            raise RuntimeError(f"CouchDB respondió HTTP {error.code} en {method} {path}") from error


def main() -> int:
    try:
        username = os.environ.get("API_COUCHDB_USERNAME", "mercapos_api")
        password = os.environ["API_COUCHDB_PASSWORD"]
        database = os.environ.get("COUCHDB_DATABASE", "mercapos")
        auth_database = os.environ.get("COUCHDB_AUTH_DATABASE", "mercapos_auth")
        role = "mercapos_writer"
        couch = Couch()
        user_id = "org.couchdb.user:" + username
        user_path = "/_users/" + quote(user_id, safe="")
        current = couch.request("GET", user_path, allow=(404,))
        if current is None:
            couch.request("PUT", user_path, {
                "_id": user_id,
                "name": username,
                "type": "user",
                "roles": [role],
                "password": password,
            }, allow=(409,))
            current = couch.request("GET", user_path)
        if current.get("name") != username or current.get("type") != "user":
            raise RuntimeError("El documento existente no corresponde a la cuenta de servicio")
        roles = set(current.get("roles", []))
        if roles != {role}:
            raise RuntimeError("La cuenta existente tiene roles distintos; revise manualmente antes de continuar")

        for target in dict.fromkeys((database, auth_database)):
            db = quote(target, safe="")
            # Create the dedicated auth database if needed. A 412/409 only means
            # "already exists" here; the following GET proves it is readable.
            couch.request("PUT", f"/{db}", allow=(409, 412))
            security = couch.request("GET", f"/{db}/_security")
            members = security.get("members") or {}
            names = set(members.get("names") or [])
            member_roles = set(members.get("roles") or [])
            member_roles.add(role)
            updated = dict(security)
            updated["members"] = {
                **members,
                "names": sorted(names),
                "roles": sorted(member_roles),
            }
            if updated != security:
                couch.request("PUT", f"/{db}/_security", updated)
        print(f"Cuenta {username} verificada; rol {role} habilitado en las bases de negocio y autenticación.")
        print("Configure la API con esta cuenta; conserve la cuenta admin solo para instalación.")
        return 0
    except KeyError as error:
        print(f"Falta variable de entorno: {error.args[0]}", file=sys.stderr)
    except Exception as error:  # never include request headers or passwords in errors
        print(f"No se pudo preparar la cuenta de servicio: {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
