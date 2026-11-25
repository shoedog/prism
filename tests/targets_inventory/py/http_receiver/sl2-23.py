import httpx
async def make() -> httpx.AsyncClient:
    return httpx.AsyncClient()
async def f(u):
    client = httpx.AsyncClient()
    client = make()
    return await client.get(u)
