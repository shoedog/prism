import requests


@deco
def fetch(u):
    return requests.get(u)
