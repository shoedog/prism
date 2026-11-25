from requests import Session
def fetch(u):
    (client := Session())
    return client.get(u)
