import requests
def f(modules):
    return [[item for item in requests.get("u")]
            for requests in modules]
