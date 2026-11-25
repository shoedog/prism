from requests import Session
def fetch(u):
    with Session() as client:
        result = client.get(u)
        del client
    return result
