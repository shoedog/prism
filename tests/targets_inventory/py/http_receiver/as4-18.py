from httpx import AsyncClient
async def make() -> AsyncClient:
    return AsyncClient()
async def fetch(u):
    pending = make()
    client = await pending
    return await client.get(u)
