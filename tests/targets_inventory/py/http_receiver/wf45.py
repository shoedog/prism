from requests import Session
def fetch(seed, replacement, u):
    client: Session = seed
    alias = client
    alias.get = replacement
    return client.get(u)
