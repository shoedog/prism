import requests
def f(value, u, C):
    match value:
        case C(requests=None):
            return requests.get(u)
