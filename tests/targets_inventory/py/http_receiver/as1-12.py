from typing import cast
from requests import Session
def fetch(raw, u):
    client = cast(Session, raw)
    return client.get(u)
