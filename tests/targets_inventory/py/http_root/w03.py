import requests


def fetch(a, u):
    return wrap(a), requests.get(u)
