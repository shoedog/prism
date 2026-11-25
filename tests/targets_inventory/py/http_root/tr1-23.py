import requests
def f():
    return (requests.get("u") for _ in range(1))
