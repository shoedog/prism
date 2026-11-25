class C:
    def __init__(self):
        from requests import Session as Client
        self.session = Client()
    def f(self, u, Client):
        return self.session.get(u)
