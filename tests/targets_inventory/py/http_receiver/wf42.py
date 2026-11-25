from requests import Session
class API:
    def __init__(self):
        self.session = Session()
    def fetch(self, u, replacement):
        vars(self)["session"] = replacement
        return self.session.get(u)
