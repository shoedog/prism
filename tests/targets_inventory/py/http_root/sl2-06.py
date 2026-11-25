import requests
def f(u):
    callback = lambda value: (requests := value)
    return requests.get(u)
