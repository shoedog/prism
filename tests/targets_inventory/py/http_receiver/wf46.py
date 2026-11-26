from requests import Session
def fetch(client: Session, u, replacement):
    def reset():
        nonlocal client
        client = replacement
    reset()
    return client.get(u)
