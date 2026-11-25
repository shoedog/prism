import requests


def make() -> requests.Session:
    yield requests.Session()


def fetch(u):
    s = make()
    return s.get(u)
