import requests
def fetch(status, u):
    match status:
        case requests.codes.ok:
            return requests.get(u)
