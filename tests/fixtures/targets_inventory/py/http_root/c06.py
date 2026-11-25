import requests


def fetch(urls):
    xs = [requests.get(x) for x in urls]
    return xs
