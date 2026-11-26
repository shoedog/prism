import requests


def encode(x):
    return requests.utils.quote(x)
