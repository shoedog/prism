import sys
from pathlib import Path

import pytest
from tier_a.lsp_client import LspClient, LspServerError, LspTimeout

ECHO = [sys.executable, str(Path(__file__).parent / "echo_server.py")]


@pytest.fixture
def client():
    c = LspClient(ECHO, cwd=".", default_timeout=5.0)
    c.start()
    yield c
    c.stop()


def test_request_response_roundtrip(client):
    assert client.request("test/echo", {"x": [1, 2]}) == {"x": [1, 2]}


def test_concurrent_correlation(client):
    # interleaved ids must route to the right callers
    assert client.request("test/echo", {"n": 1}) == {"n": 1}
    assert client.request("test/echo", {"n": 2}) == {"n": 2}


def test_timeout_raises_and_client_survives(client):
    with pytest.raises(LspTimeout):
        client.request("test/slow", {}, timeout=0.2)
    assert client.request("test/echo", {"after": True}) == {"after": True}


def test_server_error_raises(client):
    with pytest.raises(LspServerError):
        client.request("test/error", {})


def test_notifications_are_captured(client):
    client.request("test/notifyme", {})
    notes = client.drain_notifications()
    assert any(n["method"] == "test/notification" for n in notes)
