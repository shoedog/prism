"""Qualification evidence is a reviewed kit artifact: loaded from the kit tree and pinned."""

from __future__ import annotations

import shutil
import unittest
from unittest.mock import patch

from measure import oracle_inputs
from measure.edges import canonical

from .helpers import write_tree
from . import test_oracle as fixtures

BINARY = "e" * 64


class Pin(unittest.TestCase):
    def setUp(self):
        self.h = fixtures.OracleTests()
        self.addCleanup(self.h.doCleanups)
        self.h.setUp()
        self.ts = self.h.typescript()

    def evidence(self, root):
        """Write builder-shaped evidence under `root`; return the bundle digest."""
        raw, = fixtures.read_jsonl(fixtures.CASES / "oracle-022/probe.jsonl")
        record = canonical(fixtures.synthetic_evidence(BINARY, "named-function", raw))
        bundle = canonical(dict(schema="prism.measure.qualification/1", builds={BINARY: {
            "named-function": dict(path="evidence.json", sha256=oracle_inputs.digest(record))}}))
        write_tree(root, {"bundle.json": bundle, "evidence.json": record})
        return oracle_inputs.digest(bundle)

    def kit(self, *, installed=True, pin="matching"):
        self.copies = getattr(self, "copies", 0) + 1
        kit = self.h.work / f"kit-{self.copies}"
        shutil.copytree(fixtures.KIT, kit)
        shutil.rmtree(kit / "qualifications", ignore_errors=True)
        digest = self.evidence(kit / "qualifications") if installed else None
        value = digest if pin == "matching" else pin
        if installed is False and pin == "matching":
            value = None
        (kit / "qualification-pin.json").write_bytes(canonical(
            dict(schema="prism.measure.qualification-pin/1", bundle_sha256=value)) + b"\n")
        return kit

    def prepare(self, kit, **options):
        with patch.object(oracle_inputs, "KIT", kit), \
                patch.object(oracle_inputs, "QUALIFICATIONS", kit / "qualifications"):
            return oracle_inputs.prepare_inputs(self.ts, **options)

    def refused(self, kit, pattern, **options):
        with self.assertRaisesRegex(oracle_inputs.OracleError, pattern) as caught:
            self.prepare(kit, **options)
        self.assertEqual(caught.exception.code, 3)

    def test_caller_supplied_evidence_outside_the_kit_tree_is_refused(self):
        kit = self.kit()
        outside = self.h.work / "outside"
        self.evidence(outside)
        self.refused(kit, "qualification-provenance: evidence outside the kit tree",
                     qualification_dir=outside)
        # The same bytes installed in the kit tree and pinned qualify the build.
        self.assertEqual(self.prepare(kit)["builds"], {BINARY: ["named-function"]})
        self.assertEqual(self.prepare(kit, qualification_dir=kit / "qualifications")["builds"],
                         {BINARY: ["named-function"]})

    def test_installed_bundle_must_match_the_kit_pin(self):
        kit = self.kit(pin="0" * 64)
        self.refused(kit, "qualification-pin: bundle digest differs from the kit pin")
        control = self.kit()
        self.assertEqual(self.prepare(control)["builds"], {BINARY: ["named-function"]})
        # Edited evidence behind an unchanged pin is the same refusal.
        edited = self.kit()
        bundle = edited / "qualifications/bundle.json"
        bundle.write_bytes(bundle.read_bytes() + b"\n")
        self.refused(edited, "qualification-pin: bundle digest differs from the kit pin")

    def test_pin_and_directory_must_agree_on_presence(self):
        self.refused(self.kit(installed=True, pin=None),
                     "qualification-pin: evidence is installed but the kit pins none")
        absent = self.kit(installed=False, pin="0" * 64)
        self.refused(absent, "qualification-pin: pinned bundle is not installed")
        self.assertEqual(self.prepare(self.kit(installed=False))["builds"], {})

    def test_pin_file_is_closed_and_part_of_the_selfcheck_key(self):
        kit = self.kit()
        key = self.prepare(kit)["key"]
        pin = kit / "qualification-pin.json"
        pin.write_bytes(canonical(dict(schema="prism.measure.qualification-pin/1",
                                       bundle_sha256=self.prepare(kit)["qualification_sha256"],
                                       extra=1)))
        self.refused(kit, "qualification-pin: invalid pin")
        pin.write_bytes(canonical(dict(schema="prism.measure.qualification-pin/1",
                                       bundle_sha256="not-a-digest")))
        self.refused(kit, "qualification-pin: invalid pin")
        pin.unlink()
        self.refused(kit, "selfcheck-binding")
        other = self.kit(installed=False)
        self.assertNotEqual(self.prepare(other)["key"], key)


if __name__ == "__main__":
    unittest.main()
