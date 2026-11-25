import requests
from typing import Optional
def make() -> Optional[requests.Session]:
    return requests.Session()
def f():
    client = make()
    return client.get("u")
