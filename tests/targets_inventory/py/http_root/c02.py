import requests


def outer(u):
    @deco(
        requests.get(u))
    def inner():
        pass

    return inner
