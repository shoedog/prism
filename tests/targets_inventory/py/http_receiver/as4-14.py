from requests import Session
class API:
    def __init__(self):
        self.client = Session()
    def callback(self):
        def fetch(u):
            return self.client.get(u)
        return fetch
