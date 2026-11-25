from requests import Session
from httpx import Client
def f(session: Session | Client, u):
    return session.get(u)
