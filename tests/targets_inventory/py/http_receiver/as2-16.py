from requests import Session
def fetch(u):
    client = Session()
    def configure(client):
        def clear():
            nonlocal client
            del client
    return client.get(u)
