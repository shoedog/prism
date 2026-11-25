def outer():
    class C:
        import requests
        values = [requests.get("u") for _ in (1,)]
    return C
