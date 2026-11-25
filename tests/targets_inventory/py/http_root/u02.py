import requests


def fetch(u):
    r = parse(requests.get(u))
