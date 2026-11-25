from requests import Session
class Client(Session):
    def fetch(self, u):
        return super().get(u)
