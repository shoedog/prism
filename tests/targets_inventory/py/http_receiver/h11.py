import httpx


async def f(u):
    async with httpx.AsyncClient() as c:
        return await c.get(u)
