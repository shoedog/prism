import requests


def fetch(x, u):
    if check(x) and requests.get(u).ok:
        return True
