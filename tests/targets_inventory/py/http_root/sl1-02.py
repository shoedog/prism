import requests
def outer(u):
    class Outer:
        requests = None
        class Inner:
            value = requests.get(u)
    return Outer
