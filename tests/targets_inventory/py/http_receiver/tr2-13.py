import requests
def f():
    client = requests.Session()
    return client.post("https://example.test")
