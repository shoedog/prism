class C:
    from requests import Session
    def f(self, session: Session, u):
        return session.get(u)
