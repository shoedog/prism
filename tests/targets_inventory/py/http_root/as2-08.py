import requests
def callbacks(u):
    return (lambda: requests.get(u)), (lambda: requests.get(u))
