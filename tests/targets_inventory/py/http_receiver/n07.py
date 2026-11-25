import requests


def make():
    return requests.Session()


def fetch(u):
    s = make()
    return s.get(u)
