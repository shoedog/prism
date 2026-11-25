import aiohttp
class Client:
    async def f(self, u):
        async with aiohttp.ClientSession() as session:
            async with session.post(u) as response:
                return response
