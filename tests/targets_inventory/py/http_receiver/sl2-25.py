import httpx
async def f(client: httpx.AsyncClient, urls):
    return [await client.get(u) async for u in urls]
