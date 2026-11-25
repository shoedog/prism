import requests
def fetch(use, flag, a, b):
    return use(requests.get(a) if flag else requests.get(b))
