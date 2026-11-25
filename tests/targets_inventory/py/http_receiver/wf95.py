from requests import Session
def make():
    return Session()
def fetch(u):
    class Wrapper:
        client = make()
        response = client.get(u)
    return Wrapper.response
