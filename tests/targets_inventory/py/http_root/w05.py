import requests


def fetch(d, k, u):
    x = d[key(k)] or requests.get(u)
