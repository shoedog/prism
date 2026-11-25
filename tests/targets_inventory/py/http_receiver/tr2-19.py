import requests
def f():
    client = requests.Session()
    return client.request("https://example.test")
