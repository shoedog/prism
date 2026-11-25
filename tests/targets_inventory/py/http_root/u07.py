import requests


def fetch(u):
    return requests.get(
        u,
        timeout=1,
    )
