import os
from importlib import import_module

assert all(name not in os.environ for name in ("PYTHONPATH", "NODE_PATH", "GOPATH"))
import_module("truth_runner_ambient_decoy")
app = None
