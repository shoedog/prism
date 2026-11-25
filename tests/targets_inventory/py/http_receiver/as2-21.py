from requests import Session
class FetchMixin:
    def fetch(self: Session, u):
        return self.get(u)
