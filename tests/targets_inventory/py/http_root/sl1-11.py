import requests
def f(value, u):
    callback = lambda x=(requests := value): x
    return requests.get(u)
