#!/usr/bin/env python3
"""Crea la base interna _users si CouchDB aún no la tiene."""

import base64
import json
import os
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen

url = os.environ.get("COUCHDB_URL", "http://couchdb:5984").rstrip("/")
username = os.environ["COUCHDB_ADMIN_USERNAME"]
password = os.environ["COUCHDB_ADMIN_PASSWORD"]
token = base64.b64encode(f"{username}:{password}".encode()).decode()
headers = {"Authorization": f"Basic {token}", "Content-Type": "application/json"}

try:
    request = Request(url + "/_users", data=b"{}", method="PUT", headers=headers)
    with urlopen(request, timeout=15) as response:
        print("Base _users creada.")
except HTTPError as error:
    if error.code in (412, 409):
        print("Base _users ya existe.")
    else:
        print(f"No se pudo crear _users: HTTP {error.code}", file=sys.stderr)
        sys.exit(1)
