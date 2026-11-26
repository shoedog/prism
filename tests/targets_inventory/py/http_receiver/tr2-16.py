import requests
def f():
    client = requests.Session()
    return client.delete("https://example.test")
