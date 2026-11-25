from requests import Session
client = Session()
def fetch(u):
    del client
    return client.get(u)
