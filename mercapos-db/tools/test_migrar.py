import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from migrar import audit, doc_url_id, fixture_documents, materialize, write_destination


ROOT = Path(__file__).resolve().parents[1]


class MigrationFixtureTests(unittest.TestCase):
    def test_url_design_document_preserves_required_path_separator(self):
        self.assertEqual(doc_url_id("_design/validacion_v2"), "_design/validacion_v2")
        self.assertEqual(doc_url_id("producto:uno"), "producto%3Auno")

    def load(self, folder):
        return fixture_documents([folder / name for name in
            ("02-documentos.http", "06-desconexion.http", "09-operaciones-adicionales.http")])

    def test_fixture_corregido_conserva_saldos(self):
        source, preliminary = self.load(ROOT / "http")
        proposed, anomalies, _, before, after = audit(source)
        self.assertEqual(preliminary + anomalies, [])
        self.assertEqual(before[("puesto:0001", "producto:0042")], 63)
        self.assertEqual(before[("puesto:0001", "producto:0107")], 29)
        self.assertEqual(after, before)
        self.assertFalse(any(doc.get("tipo") == "movimiento" for doc in proposed))
        self.assertEqual(len([d for d in proposed if d.get("tipo") == "operacion"]), 6)

    def test_fixture_original_bloquea_referencia_ambigua(self):
        source, _ = self.load(ROOT / "fixtures" / "legacy")
        _, anomalies, _, _, _ = audit(source)
        self.assertTrue(any("referencia de devolución inválida" in item for item in anomalies))

    def test_dos_formatos_duplicados_no_se_deduplican(self):
        source, _ = self.load(ROOT / "http")
        old = next(d for d in source if d["_id"] == "mov:550e8400-e29b-41d4-a716-446655440000:r1")
        duplicate = dict(old, _id="mov:550e8400-e29b-41d4-a716-446655440000:r1:0042")
        _, anomalies, _, _, _ = audit(source + [duplicate])
        self.assertTrue(any("movimientos adicionales o duplicados" in item or "se esperó 1 movimiento" in item for item in anomalies))

    def test_documentos_propuestos_tienen_cantidades_numericas(self):
        source, _ = self.load(ROOT / "http")
        proposed, anomalies, _, _, _ = audit(source)
        self.assertEqual(anomalies, [])
        for doc in materialize(proposed):
            for effect in doc.get("impactos_inventario", []):
                self.assertIsInstance(effect["cantidad"], (int, float))

    def test_reintento_de_escritura_no_cambia_documentos(self):
        source, _ = self.load(ROOT / "http")
        proposed, anomalies, _, _, _ = audit(source)
        self.assertEqual(anomalies, [])

        class FakeCouch:
            username = "admin"
            def __init__(self):
                self.docs = {}
            def request(self, method, path, body=None, allowed=()):
                if path.endswith("/_security") or path.endswith("/_index"):
                    return {"ok": True}
                if method == "GET":
                    return self.docs.get(path)
                if method == "PUT" and body is not None:
                    self.docs[path] = dict(materialize(body), _rev="1-prueba")
                return {"ok": True}

        couch = FakeCouch()
        write_destination(couch, "mercapos_test_migracion", proposed)
        first = dict(couch.docs)
        write_destination(couch, "mercapos_test_migracion", proposed)
        self.assertEqual(couch.docs, first)


if __name__ == "__main__":
    unittest.main()
