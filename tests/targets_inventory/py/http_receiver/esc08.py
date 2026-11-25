from httpx import AsyncClient
async def enter(self):
    return {}
AsyncClient.__aenter__ = enter
async def fetch(u):
    async with AsyncClient() as s:
        return await s.get(u)
