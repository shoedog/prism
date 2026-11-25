import requests


def fetch(u):
    s: requests.Session = other()
    return s.get(u)
