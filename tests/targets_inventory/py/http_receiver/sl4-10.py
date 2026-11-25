import requests
class C:
    def __init__(self):
        self.session = requests.Session()
    def replace(self, value):
        setattr(self, "ses" "sion", value)
    def fetch(self, u):
        return self.session.get(u)
