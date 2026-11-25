from builtins import list as Optional
from requests import Session
def fetch(client: Optional[Session], u):
    return client.get(u)
