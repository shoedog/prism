import httpx
def f():
    client = httpx.AsyncClient()
    return client.stream("https://example.test")
