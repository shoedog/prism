def fetch(u):
    try:
        from requests import Session as Client
    except ImportError:
        from httpx import Client
    client = Client()
    return client.get(u)
