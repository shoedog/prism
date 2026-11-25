from requests import Session as Client
def f(session: Client, u):
    from httpx import Client
    return session.get(u)
