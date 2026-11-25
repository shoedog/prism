import requests
def outer():
    def inner(a=object(), b=requests.get("default")): return requests.get("body")
