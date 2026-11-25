from __future__ import annotations
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from requests import Session
def fetch(client: Session, u):
    return client.get(u)
