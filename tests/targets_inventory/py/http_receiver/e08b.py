import requests


def outer(u):
    s = requests.Session()

    def replace():
        nonlocal s
        s = other()

    return s.get(u)
