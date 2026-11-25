import requests
class Outer:
    def __init__(self):
        self.session = requests.Session()
    class Inner:
        def __init__(self, value):
            self.session = value
    def f(self, u):
        return self.session.get(u)
