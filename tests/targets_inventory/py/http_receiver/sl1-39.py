from requests import Session as Client
def make() -> Client:
    return Client()
def f(u):
    from httpx import Client
    session = make()
    return session.get(u)
