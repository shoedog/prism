from __future__ import annotations
from httpx import Client
from requests import Session
def make() -> "Client":
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
