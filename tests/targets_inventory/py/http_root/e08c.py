import requests


def fetch(u, value):
    match value:
        case requests:
            return requests.get(u)
