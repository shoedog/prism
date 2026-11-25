from requests import Session
class API:
    def snapshot(self, u):
        self.client = Session()
        class Result:
            reply = self.client.get(u)
