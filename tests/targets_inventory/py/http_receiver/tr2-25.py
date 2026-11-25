import httpx
def f():
    client = httpx.AsyncClient()
    return client.post("https://example.test")
