from requests import Session
def f(u, client=Session()):
    return client.get(u)
