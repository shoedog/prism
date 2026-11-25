from . import requests


def fetch(u):
    return requests.get(u)
