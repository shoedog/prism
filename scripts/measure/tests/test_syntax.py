from __future__ import annotations

import ast
import unittest

from .helpers import MEASURE, REPO


def parse_as_python_39(source, filename="<fixture>"):
    return ast.parse(source, filename=filename, feature_version=(3, 9))


class SyntaxTests(unittest.TestCase):
    def test_every_module_parses_as_python_39(self):
        self.assertTrue((MEASURE / "__main__.py").is_file(), "kit entry point is missing")
        for root in (MEASURE, REPO / "scripts" / "memguard"):
            for path in root.rglob("*.py"):
                with self.subTest(path=path.relative_to(REPO)):
                    parse_as_python_39(path.read_text(encoding="utf-8"), str(path))
