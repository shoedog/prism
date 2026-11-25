import aiohttp
def f():
    client = aiohttp.ClientSession()
    return client.request("https://example.test")
