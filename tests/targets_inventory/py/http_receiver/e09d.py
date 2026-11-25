import requests


def make() -> requests.Session:
    return requests.Session()


def fetch(u, make):
    s = make()
    return s.send(u)
