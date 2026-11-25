class C:
    from requests import Session as Client
    def factory(self, *, session: Client):
        def fetch(u):
            return session.get(u)
        return fetch
