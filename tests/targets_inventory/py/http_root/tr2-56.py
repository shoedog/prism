def outer():
    import requests
    def inner():
        nonlocal requests
        requests = None
    return requests.get("u")
