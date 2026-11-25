import requests
async def make() -> requests.Session:
    yield requests.Session()
async def f():
    s = await make()
    return s.get("u")
