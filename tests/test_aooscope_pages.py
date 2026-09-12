import copy
import json
import tempfile
import unittest
from pathlib import Path

from aooscope.page_store import (
    PageStore,
    PageValidationError,
    RevisionConflict,
    validate_document,
)
from aooscope.factory_templates import factory_document


class AooScopePageStoreTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.store = PageStore(self.root)

    def tearDown(self):
        self.tmp.cleanup()

    def test_first_load_migrates_factory_carousel(self):
        doc = self.store.load()
        names = [doc["pages"][pid]["name"] for pid in doc["carousel"]]
        self.assertEqual(names[:5], ["Splash", "Home", "Storage", "Compute", "Media"])
        self.assertEqual(doc["schema_version"], 1)
        self.assertEqual(doc["revision"], 1)
        self.assertTrue((self.root / "pages.json").is_file())
    def test_save_requires_matching_document_revision(self):
        doc = self.store.load()
        stale = doc["revision"] - 1
        with self.assertRaises(RevisionConflict):
            self.store.save(doc, expected_revision=stale)

        updated = copy.deepcopy(doc)
        updated["pages"][doc["carousel"][1]]["name"] = "CPU Wall"
        saved = self.store.save(updated, expected_revision=doc["revision"])
        self.assertEqual(saved["revision"], doc["revision"] + 1)
        self.assertEqual(saved["pages"][doc["carousel"][1]]["name"], "CPU Wall")

    def test_restore_replaces_factory_content_but_preserves_identity_and_position(self):
        doc = self.store.load()
        home_id = doc["carousel"][1]
        edited = copy.deepcopy(doc)
        edited["pages"][home_id]["name"] = "My Home"
        edited["pages"][home_id]["layers"] = []
        saved = self.store.save(edited, expected_revision=doc["revision"])

        restored = self.store.restore(home_id)
        self.assertEqual(restored["carousel"].index(home_id), 1)
        self.assertEqual(restored["pages"][home_id]["id"], home_id)
        self.assertEqual(restored["pages"][home_id]["template_id"], "factory.home.v1")
        self.assertEqual(restored["pages"][home_id]["name"], "Home")
        self.assertGreater(len(restored["pages"][home_id]["layers"]), 0)
        self.assertGreater(restored["revision"], saved["revision"])
    def test_validation_rejects_duplicate_layer_ids(self):
        doc = factory_document()
        page = doc["pages"][doc["carousel"][1]]
        page["layers"] = [
            {"id": "same", "type": "text", "x": 10, "y": 10, "width": 100, "height": 40, "z": 1, "text": "A"},
            {"id": "same", "type": "text", "x": 20, "y": 20, "width": 100, "height": 40, "z": 2, "text": "B"},
        ]
        with self.assertRaises(PageValidationError):
            validate_document(doc)

    def test_validation_rejects_out_of_bounds_without_clip(self):
        doc = factory_document()
        page = doc["pages"][doc["carousel"][1]]
        page["layers"] = [
            {"id": "bad", "type": "text", "x": 940, "y": 20, "width": 80, "height": 40, "z": 1, "text": "A"}
        ]
        with self.assertRaises(PageValidationError):
            validate_document(doc)

        page["layers"][0]["clip"] = True
        validate_document(doc)

    def test_validation_rejects_bad_color_type_and_duration(self):
        for mutate in (
            lambda p: p.update(duration=0),
            lambda p: p["background"].update(color="red"),
            lambda p: p["layers"].append({"id":"x","type":"unknown","x":0,"y":0,"width":10,"height":10,"z":1}),
        ):
            doc = factory_document()
            mutate(doc["pages"][doc["carousel"][1]])
            with self.assertRaises(PageValidationError):
                validate_document(doc)
