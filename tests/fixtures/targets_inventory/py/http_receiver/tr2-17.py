import requests
def f():
    client = requests.Session()
    return client.head("https://example.test")
