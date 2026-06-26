# conftest.py — unit test environment for adoption package
# ToolCorrectnessMetric (deepeval 4.x) eagerly initialises an OpenAI client at
# construction time, even though it is used deterministically (no LLM call).
# Set a dummy key so the import-time GATE_METRICS list can be created in unit
# tests that run without real OpenAI credentials.
import os
os.environ.setdefault("OPENAI_API_KEY", "sk-dummy-unit-test-key")
