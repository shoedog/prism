import requests


def f(s: "requests.Session", u):
    return s.get(u)
