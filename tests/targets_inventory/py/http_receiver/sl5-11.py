from httpx import Client
from requests import Session
def f(u):
    client: Client = Session()
    return client.get(u)
