import requests
def reset(change, replacement):
    if change:
        requests.Session = replacement
def direct(u):
    return requests.Session().get(u)
def local(u):
    session = requests.Session()
    return session.get(u)
def managed(u):
    with requests.Session() as session:
        return session.get(u)
class API:
    def __init__(self):
        self.session = requests.Session()
    def fetch(self, u):
        return self.session.get(u)
def annotated(session: "requests.Session", u):
    return session.get(u)
def make() -> "requests.Session":
    return requests.Session()
def factory(u):
    session = make()
    return session.get(u)
