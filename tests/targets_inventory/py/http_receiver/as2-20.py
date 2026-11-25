from typing import Optional as Maybe
from requests import Session
def fetch(client: Maybe[Session], u):
    return client.get(u)
