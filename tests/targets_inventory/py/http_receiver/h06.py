import requests


def fetch(u):
    with requests.Session() as s:
        return s.get(u)
