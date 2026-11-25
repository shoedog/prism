import requests


def good(u):
    return requests.get(u)


def bad(u):
    r = requests.get(u)
    x = = 1
    return r
