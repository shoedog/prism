import requests


def fetch(u):
    xs = [r for r in requests.get(u)]
    return xs
