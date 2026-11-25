import requests


def fetch(u):
    return requests.get(build(u))
