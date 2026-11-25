import httpx
def f():
    client = httpx.AsyncClient()
    return client.request("https://example.test")
