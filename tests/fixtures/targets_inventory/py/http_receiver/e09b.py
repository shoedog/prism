import httpx


async def make() -> httpx.AsyncClient:
    return httpx.AsyncClient()


async def fetch(u):
    c = make()
    return await c.get(u)
