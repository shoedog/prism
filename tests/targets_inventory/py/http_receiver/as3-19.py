from typing import Annotated
from requests import Session
def fetch(client: Annotated[Session, "transport"], u):
    return client.get(u)
