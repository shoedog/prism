import requests
def f(manager, u):
    with (manager, requests.Session() as session):
        return session.get(u)
