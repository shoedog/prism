from requests import Session
def fetch(obj, u):
    match obj:
        case Session() as client:
            return client.get(u)
