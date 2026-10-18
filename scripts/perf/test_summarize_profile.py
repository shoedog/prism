import unittest

from summarize_profile import render


class RendererTest(unittest.TestCase):
    def test_empty_owner_population_and_unavailable_measurements(self):
        output = render([
            {"record": "header", "schema_version": 1, "top_n": 10},
            {"record": "phase", "name": "cache.load", "file": None,
             "time_kind": "serial_wall", "status": "not_run", "wall_ns": 0,
             "cpu_ns": None, "peak_rss_bytes": None},
            {"record": "summary", "owner_count": 0, "top_dfg_share": 0,
             "top_owners": [], "other_owners": dict.fromkeys(
                 ["count", "total_ns", "p50_ns", "p90_ns", "p99_ns", "max_ns"], 0)},
        ])
        self.assertIn("cache.load | serial_wall | not_run | 0.000 | - | -", output)
        self.assertIn("Owners: 0; top-10 share of owner DFG work: 0.00%", output)

    def test_future_schema_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "unsupported"):
            render([{"record": "header", "schema_version": 2}])

    def test_same_line_duplicate_names_keep_their_own_metadata(self):
        owners = [dict(record="owner", file="a.js", owner="f", start_line=1,
                       start_byte=byte, total_ns=1_000_000, language="JavaScript",
                       flow_search_ns=0, reaching_definitions_ns=0, byte_size=size,
                       ast_nodes=nodes, peak_rss_bytes=4 * 2**20) for byte, size, nodes in [(0, 15, 7), (0, 30, 12)]]
        output = render([
            {"record": "header", "schema_version": 1, "top_n": 10}, *owners,
            {"record": "summary", "owner_count": 2, "top_dfg_share": 1,
             "top_owners": owners, "other_owners": dict.fromkeys(
                 ["count", "total_ns", "p50_ns", "p90_ns", "p99_ns", "max_ns"], 0)},
        ])
        self.assertIn("| 15 | 7 |", output)
        self.assertIn("| 30 | 12 |", output)
        self.assertIn("| 30 | 12 | 4.00 |", output)


if __name__ == "__main__":
    unittest.main()
