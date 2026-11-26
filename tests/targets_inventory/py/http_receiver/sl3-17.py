import requests
async def make(items) -> requests.Session:
    [item async for item in items]
    return requests.Session()
async def f(items, u):
    session = await make(items)
    return session.get(u)
