import requests


def outer(u):
    def inner():
        return requests.get(u)
        # End of the nested body.

    return inner
