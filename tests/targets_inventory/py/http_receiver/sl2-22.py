import httpx
import requests
def make() -> requests.Session:
    return requests.Session()
def f(u):
    client: httpx.Client = make()
    return client.get(u)
