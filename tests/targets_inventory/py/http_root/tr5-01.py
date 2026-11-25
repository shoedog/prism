import requests
def outer():
    def inner(requests=requests.get("u")):
        return requests
    return inner
