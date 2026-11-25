import requests


def make():
    f = lambda u: requests.get(u)
