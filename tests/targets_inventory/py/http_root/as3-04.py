import requests
def fetch(u):
    return getattr(requests, "get")(u)
