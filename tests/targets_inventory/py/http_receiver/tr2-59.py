import requests
def make() -> requests.Session:
    return requests.Session()
def make() -> requests.Session:
    return requests.Session()
def f():
    client = make()
    return client.get("u")
