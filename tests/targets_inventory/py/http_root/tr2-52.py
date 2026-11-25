import requests
def outer():
    requests = object()
    def inner():
        import requests
        return requests.get("u")
    return inner
