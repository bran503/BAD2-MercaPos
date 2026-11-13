import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from instalar_esquema import install


class FakeCouch:
    def __init__(self):
        self.docs = {
            "/mercapos_test_schema/_design/inventario": {
                "_id": "_design/inventario",
                "_rev": "1-existing",
                "views": {"vista_personalizada": {"map": "function (doc) { emit(doc._id, 1); }"}},
            }
        }
        self.calls = []

    def request(self, method, path, body=None, allowed=()):
        self.calls.append((method, path))
        if method == "GET" and path == "/mercapos_test_schema":
            return {"db_name": "mercapos_test_schema"}
        if method == "GET":
            return self.docs.get(path)
        if method == "PUT":
            revision = str(len(self.calls)) + "-test"
            self.docs[path] = dict(body, _rev=revision)
            return {"ok": True, "rev": revision}
        if method == "POST" and path == "/mercapos_test_schema/_index":
            return {"result": "created"}
        raise AssertionError((method, path))


class InstallSchemaTests(unittest.TestCase):
    def test_instala_y_repite_sin_escribir_documentos_o_security(self):
        couch = FakeCouch()
        install(couch, "mercapos_test_schema")
        first_docs = dict(couch.docs)
        first_puts = [call for call in couch.calls if call[0] == "PUT"]
        self.assertEqual(len(first_puts), 2)
        self.assertEqual(set(first_docs), {
            "/mercapos_test_schema/_design/inventario",
            "/mercapos_test_schema/_design/validacion_v2",
        })
        self.assertIn("vista_personalizada", first_docs["/mercapos_test_schema/_design/inventario"]["views"])
        self.assertFalse(any(path.endswith("/_security") for _, path in couch.calls))
        install(couch, "mercapos_test_schema")
        second_puts = [call for call in couch.calls if call[0] == "PUT"]
        self.assertEqual(first_puts, second_puts)


if __name__ == "__main__":
    unittest.main()
