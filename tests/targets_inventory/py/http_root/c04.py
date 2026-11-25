import requests


def outer(u):
    class Client:
        response = requests.get(u)

    return Client
