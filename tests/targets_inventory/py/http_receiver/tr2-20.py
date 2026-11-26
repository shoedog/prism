import httpx
def f():
    client = httpx.Client()
    return client.get("https://example.test")
