import requests


def fetch(p, u):
    with open(p) as f, requests.get(u) as r:
        pass
