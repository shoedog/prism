def outer():
    class C:
        import requests
        values = [x for x in requests.get("u")]
    return C
