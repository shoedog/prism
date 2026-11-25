import requests
def f():
    return [requests for _ in (0,)
            for requests in requests.get("u")]
