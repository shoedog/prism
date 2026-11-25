import requests


def fetch(u, pool):
    s = requests.Session()
    for s in pool:
        pass
    return s.get(u)
