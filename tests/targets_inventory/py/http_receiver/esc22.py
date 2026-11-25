from requests import Session
def make() -> Session:
    return Session()
def fetch(use,u):
    direct = Session()
    helper = make()
    return use(direct.get(u), helper.get(u))
