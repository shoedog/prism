import requests
def f(value, u):
    match value:
        case _ if requests.get(u):
            return True
