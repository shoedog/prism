import requests


def outer(u):
    class Client:
        def m(self):
            return requests.get(u)

    return Client
