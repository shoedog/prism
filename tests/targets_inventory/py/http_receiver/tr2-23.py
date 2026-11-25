import httpx
def f():
    client = httpx.Client()
    return client.send("https://example.test")
