import requests
def f():
    client = requests.Session()
    return client.patch("https://example.test")
