from typing import TypeAlias
from requests import Session
Client: TypeAlias = Session
def fetch(client: Client, u):
    return client.get(u)
