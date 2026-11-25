from requests import Session as Client
def factory():
    session = Client()
    def fetch(u):
        from httpx import Client
        return session.get(u)
    return fetch
