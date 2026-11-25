import requests
class C:
    def init(self):
        self.client = requests.Session()
    def f(self, manager=None, other=None):
        self.client: object = None
        return self.client.get("u")
