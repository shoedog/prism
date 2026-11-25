import sys
from importlib import import_module

sys.path.insert(0, EXTERNAL_DIRECTORY)
import_module("external_truth_helper")
app = None
