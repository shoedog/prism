import requests


def both(u):
    a = requests.get(u)
    b = requests.get(u)
    return a, b
