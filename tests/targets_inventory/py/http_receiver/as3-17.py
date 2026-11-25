from requests import Session
def __make() -> Session:
    return Session()
class API:
    def __make(self):
        pass
def fetch(u):
    client = __make()
    return client.get(u)
