import requests


def fetch(u):
    return requests.Session().get(u)
