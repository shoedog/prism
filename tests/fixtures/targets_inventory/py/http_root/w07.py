import requests


def fetch(u):
    r = requests.get(u).json()
