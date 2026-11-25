import requests


def fetch(u, v):
    a, b = requests.get(u), requests.get(v)
