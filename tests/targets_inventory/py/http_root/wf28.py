from requests import get as fetch_http
def clear():
    del globals()["fetch_http"]
def fetch(u):
    return fetch_http(u)
