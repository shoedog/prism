import requests
def f():
    client = requests.Session()
    return client.put("https://example.test")
