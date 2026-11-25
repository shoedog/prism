import requests
async def f(clients, u):
    session = requests.Session()
    async for session in clients:
        pass
    return session.get(u)
