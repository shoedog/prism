import requests


def f(s: requests.Session, u):
    s = wrap(s)
    return s.get(u)
