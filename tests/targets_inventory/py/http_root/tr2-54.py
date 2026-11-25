def outer():
    import requests
    def inner():
        nonlocal requests
        return requests.get("u")
    return inner
