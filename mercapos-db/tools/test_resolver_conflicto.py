import sys
import unittest
from pathlib import Path
from urllib.parse import unquote

sys.path.insert(0, str(Path(__file__).resolve().parent))
from resolver_conflicto import resolve


class FakeCouch:
    def __init__(self):
        self.main_rev = "3-main"
        self.conflicts = ["3-a", "3-b"]
        self.docs = {
            "3-main": {"_id": "producto:1", "_rev": "3-main", "tipo": "producto", "nombre": "principal"},
            "3-a": {"_id": "producto:1", "_rev": "3-a", "tipo": "producto", "nombre": "elegida"},
            "3-b": {"_id": "producto:1", "_rev": "3-b", "tipo": "producto", "nombre": "otra"},
        }
        self.deleted = []

    def request(self, method, path, body=None, allowed=()):
        if method == "GET":
            if "?rev=" in path:
                rev = unquote(path.split("?rev=", 1)[1])
                return dict(self.docs[rev])
            result = dict(self.docs[self.main_rev])
            if self.conflicts:
                result["_conflicts"] = list(self.conflicts)
            return result
        if method == "PUT":
            self.main_rev = "4-main"
            self.docs[self.main_rev] = dict(body, _rev=self.main_rev)
            return {"ok": True, "rev": self.main_rev}
        if method == "DELETE":
            rev = unquote(path.split("?rev=", 1)[1])
            self.deleted.append(rev)
            self.conflicts.remove(rev)
            return {"ok": True}
        raise AssertionError(method)


class ResolverTests(unittest.TestCase):
    def test_resuelve_todas_las_hojas_incluida_la_elegida(self):
        couch = FakeCouch()
        resolve(couch, "mercapos", "producto:1", "3-a", apply=True)
        self.assertEqual(set(couch.deleted), {"3-a", "3-b"})
        self.assertEqual(couch.conflicts, [])
        self.assertEqual(couch.docs[couch.main_rev]["nombre"], "elegida")

    def test_simulacion_no_escribe(self):
        couch = FakeCouch()
        resolve(couch, "mercapos", "producto:1", "3-a", apply=False)
        self.assertEqual(couch.main_rev, "3-main")
        self.assertEqual(couch.deleted, [])


if __name__ == "__main__":
    unittest.main()
