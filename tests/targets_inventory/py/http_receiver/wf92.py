from requests import Session
client = Session()
def fetch(u):
    return client.get(u)
