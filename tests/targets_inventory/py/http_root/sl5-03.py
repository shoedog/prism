import requests
def f(items, u):
    match items:
        case [*requests]:
            return requests.get(u)
