from requests import Session
def fetch(other, u):
    requests = Session()
    requests = other
    return requests.get(u)
