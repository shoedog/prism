from requests import Session
def fetch(u):
    class Wrapper:
        client = Session()
        response = client.get(u)
    return Wrapper.response
