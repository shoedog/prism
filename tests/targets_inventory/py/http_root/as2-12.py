import requests
def evict(cache, u):
    del cache[requests.get(u)]
