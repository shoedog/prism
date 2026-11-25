from requests import get as fetch


def load(u):
    return fetch(u)
