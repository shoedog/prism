import requests
def outer():
    def inner(x: requests.get("u")):
        pass
    return inner
