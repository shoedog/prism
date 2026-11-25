import httpx
def f(u):
    client = httpx.Client()
    return client.patch(u)
