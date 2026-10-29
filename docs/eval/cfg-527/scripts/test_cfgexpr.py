import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cfgexpr as cx  # noqa: E402


def p(s):
    f, _ = cx.parse_pred(cx.tokenize(s), 0)
    return cx.norm(f)


class CfgExprTest(unittest.TestCase):
    def test_parse_and_render(self):
        self.assertEqual(cx.render(p('all(unix, feature = "x")')), 'all(feature="x",unix)')
        self.assertEqual(cx.render(p("not(not(test))")), "test")
        self.assertEqual(cx.render(p("all(a, all(b, c))")), "all(a,b,c)")
        self.assertEqual(cx.render(p("any()")), "any()")

    def test_conjuncts(self):
        self.assertEqual([cx.render(c) for c in cx.conjuncts(p("all(test, unix)"))], ["test", "unix"])

    def test_eval(self):
        env = cx.Env({("unix", None), ("target_os", "linux")}, {"std"}, {"tokio_unstable"})
        self.assertIs(cx.ev(p("unix"), env), True)
        self.assertIs(cx.ev(p("windows"), env), False)
        self.assertIs(cx.ev(p('feature = "std"'), env), True)
        self.assertIs(cx.ev(p("test"), env), False)
        self.assertIs(cx.ev(p("debug_assertions"), env), True)
        self.assertIsNone(cx.ev(p("tokio_unstable"), env))
        self.assertIs(cx.ev(p("any(tokio_unstable, unix)"), env), True)
        self.assertIs(cx.ev(p("all(tokio_unstable, windows)"), env), False)
        self.assertIs(cx.ev(p("any()"), env), False)
        self.assertIs(cx.ev(p("all()"), env), True)

    def test_static_false(self):
        self.assertTrue(cx.contains_false(p("all(x, any())")))
        self.assertFalse(cx.contains_false(p("any(x, any())")))

    def test_host_cfg(self):
        h = cx.parse_host_cfg('unix\ntarget_os="linux"\n')
        self.assertEqual(h, {("unix", None), ("target_os", "linux")})


if __name__ == "__main__":
    unittest.main()
