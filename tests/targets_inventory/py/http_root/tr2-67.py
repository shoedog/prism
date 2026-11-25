import requests
def outer():
    def inner() -> requests.get("u"):
        pass
    return inner
