import httpx
def f():
    client = httpx.Client()
    return client.request("https://example.test")
