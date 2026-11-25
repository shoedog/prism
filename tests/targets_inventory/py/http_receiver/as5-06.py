from requests import Session
def fetch(provider, u):
    direct = make()
    indirect = provider.make()
    direct.get(u)
    return indirect.get(u)
def make() -> Session:
    return Session()
