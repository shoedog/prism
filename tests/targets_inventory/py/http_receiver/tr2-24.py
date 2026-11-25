import httpx
def f():
    client = httpx.Client()
    return client.stream("https://example.test")
