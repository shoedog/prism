import requests
def f():
    return requests.get("u")

def g():
    global requests
    requests = None
