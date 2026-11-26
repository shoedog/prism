import httpx
def f():
    client = httpx.AsyncClient()
    return client.send("https://example.test")
