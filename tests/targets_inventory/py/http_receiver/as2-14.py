from contextlib import nullcontext
from requests import Session
def fetch(u):
    with nullcontext(Session()) as client:
        return client.get(u)
