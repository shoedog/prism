import aiohttp
async def f(u):
    client = aiohttp.ClientSession()
    return await client.delete(u)
