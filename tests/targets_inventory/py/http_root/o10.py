try:
    import requests
except ImportError:
    requests = None


def fetch(u):
    return requests.get(u)
