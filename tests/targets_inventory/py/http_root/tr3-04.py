import requests
def f():
    return [(lambda requests: requests.get("u"))(x) for x in ()]
