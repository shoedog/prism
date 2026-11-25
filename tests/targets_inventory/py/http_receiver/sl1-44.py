import requests
def f(u):
    session = requests.Session()
    return session.stream(u)
