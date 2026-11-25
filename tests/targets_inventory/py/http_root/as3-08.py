import requests
def fetch(ignored, u):
    return (False and ignored()) or requests.get(u)
