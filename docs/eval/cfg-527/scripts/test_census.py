import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    import cfg527_census as c  # noqa: E402
except ImportError:  # tree-sitter not installed: run under the uv command in README
    c = None


@unittest.skipIf(c is None, "tree-sitter unavailable")
class CensusTest(unittest.TestCase):
    def test_default_features(self):
        pkg = {"features": {"default": ["a", "dep:z", "y?/q"], "a": ["b", "w/x"], "b": []},
               "optional": {"w", "z", "y"}}
        self.assertEqual(c.default_features(pkg), {"a", "b", "w"})

    def test_cfg_attr_entry(self):
        info = c.AttrInfo()
        c._apply_attr(c.cx.tokenize('cfg_attr(feature = "x", cfg(any()))'), None, info)
        es = c.entries_from(info, "item")
        self.assertEqual(es[0].display, 'all(any(),feature="x")')
        self.assertEqual(c.statically_false(es), 'under:feature="x"')

    def test_path_attr(self):
        info = c.AttrInfo()
        c._apply_attr(c.cx.tokenize('cfg_attr(unix, path = "u.rs")'), None, info)
        self.assertEqual(info.paths[0][1], "u.rs")


if __name__ == "__main__":
    unittest.main()
