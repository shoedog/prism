import requests


def fetch(u):
    s = requests.Session()
    with other() as s:
        pass
    return s.get(u)
