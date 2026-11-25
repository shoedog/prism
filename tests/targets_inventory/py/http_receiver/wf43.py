from requests import Session
class API:
    def __init__(self):
        self.session = Session()
    def fetch(self, u, clear):
        result = self.session.get(u)
        if clear:
            del self.session
        return result
