import requests


def fetch(u, a, b):
    s = requests.Session()
    s, t = a, b
    return s.get(u)
