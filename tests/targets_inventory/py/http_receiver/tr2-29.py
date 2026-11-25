import aiohttp
def f():
    client = aiohttp.ClientSession()
    return client.post("https://example.test")
