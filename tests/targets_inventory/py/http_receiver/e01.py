import requests


def fetch(u, session=None):
    if session is None:
        session = requests.Session()
    return session.get(u)
