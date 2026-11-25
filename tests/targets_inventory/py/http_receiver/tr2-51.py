import requests
class C:
    def init(self):
        self.client = requests.Session()
    def f(self, manager=None, other=None):
        with manager as self.client: pass
        return self.client.get("u")
