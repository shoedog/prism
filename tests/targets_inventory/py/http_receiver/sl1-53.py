import httpx
async def f(u):
    client = httpx.AsyncClient()
    return await client.head(u)
