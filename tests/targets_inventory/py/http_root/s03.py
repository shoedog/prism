import requests


def fetch_all(urls):
    return [
        requests.get(u)
        for u in urls
    ]
