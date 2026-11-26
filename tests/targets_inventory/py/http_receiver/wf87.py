from requests import Session
def fetch(u):
    def client(seed=Session()):
        return seed
    return client.get(u)
