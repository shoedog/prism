import requests
def f(produce, u):
    session = produce()  # type: requests.Session
    return session.get(u)
