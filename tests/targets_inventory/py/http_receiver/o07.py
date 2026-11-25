import aiohttp


async def fetch(u):
    async with aiohttp.ClientSession() as s:
        async with s.get(u) as r:
            return await r.text()
