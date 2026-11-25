import requests
def fetch(obj, u):
    match obj:
        case requests.Session() if requests.get(u):
            pass
