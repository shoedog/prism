from requests import Session
class Client:
    def __init__(self):
        self.session = Session()
    def fetch(self, u):
        return self.__dict__["session"].get(u)
