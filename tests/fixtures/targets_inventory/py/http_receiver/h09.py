import requests


def make() -> requests.Session:
    return requests.Session()


def send(req):
    s = make()
    return s.send(req)
