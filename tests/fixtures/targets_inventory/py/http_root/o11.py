from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import requests


def fetch(u):
    return requests.get(u)
