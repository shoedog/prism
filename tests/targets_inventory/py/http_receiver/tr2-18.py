import requests
def f():
    client = requests.Session()
    return client.options("https://example.test")
