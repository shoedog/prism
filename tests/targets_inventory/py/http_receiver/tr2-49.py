import requests
class C:
    def init(self):
        self.client = requests.Session()
    def f(self, manager=None, other=None):
        self.client += other
        return self.client.get("u")
