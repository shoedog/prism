import httpx
def f():
    client = httpx.Client()
    return client.post("https://example.test")
