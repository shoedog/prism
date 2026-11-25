from requests import Session
def initialize(self):
    self.client = Session()
class API:
    __init__ = initialize
    def fetch(self, u):
        return self.client.get(u)
