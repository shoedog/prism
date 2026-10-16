#!/usr/bin/env python3
"""Tests for scripts/file_size.py."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from file_size import weigh  # noqa: E402


class WeighTest(unittest.TestCase):
    def test_code_comment_and_blank_lines_are_weighted_one_half_and_zero(self):
        text = 'fn a() {}\n// note\n\n/// doc\nlet x = 1; // trailing\n'
        self.assertEqual(weigh(text), (2, 2, 1, 3.0))

    def test_a_block_comment_counts_every_line_until_it_closes(self):
        text = '/* one\n   two\n   three */\nfn b() {}\n'
        self.assertEqual(weigh(text), (1, 3, 0, 2.5))

    def test_a_one_line_block_comment_does_not_swallow_the_next_line(self):
        text = '/* one line */\nfn c() {}\n'
        self.assertEqual(weigh(text), (1, 1, 0, 1.5))

    def test_empty_text_weighs_nothing(self):
        self.assertEqual(weigh(''), (0, 0, 0, 0))


if __name__ == '__main__':
    unittest.main()
