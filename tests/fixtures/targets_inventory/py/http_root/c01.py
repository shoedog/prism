import requests


def outer(u):
    def inner(x=requests.get(u)):
        return x

    return inner
