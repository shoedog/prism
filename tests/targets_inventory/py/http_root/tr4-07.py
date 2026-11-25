import requests
def f():
    def inner(x=lambda: requests.get("u")):
        pass
    return inner
