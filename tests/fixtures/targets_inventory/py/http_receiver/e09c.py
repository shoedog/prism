import httpx


async def make() -> httpx.AsyncClient:
    return httpx.AsyncClient()


async def fetch(u):
    c = await make()
    return await c.get(u)
