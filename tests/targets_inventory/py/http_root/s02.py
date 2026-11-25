import requests


def outer(u):
    def inner():
        return requests.get(u)

    return inner
