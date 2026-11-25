import requests
def f(value, u):
    match value:
        case [requests] | {"client": requests}:
            pass
    return requests.get(u)
