import requests


def fetch(u):
    with pool() as requests:
        return requests.get(u)
