from requests import Session
class API:
    def __init__(self):
        self.session = Session()
    def fetch(self, u):
        return self.session.get(u)
def replace(owner, value):
    owner.session = value
