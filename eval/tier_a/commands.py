"""Compatibility alias for command plumbing now owned by admission."""
from __future__ import annotations

import sys

from . import admission


sys.modules[__name__] = admission
