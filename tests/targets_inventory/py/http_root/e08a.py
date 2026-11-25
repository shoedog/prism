import requests


def replace(other):
    global requests
    requests = other


def fetch(u):
    return requests.get(u)
