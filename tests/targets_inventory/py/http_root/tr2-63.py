import requests
def f():
    return lambda: requests.get("u"), requests.get("v")
