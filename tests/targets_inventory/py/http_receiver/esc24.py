from requests import Session
def make(flag) -> Session:
    if flag:
        return {}
    return Session()
def fetch(flag,u):
    s = make(flag)
    return s.get(u)
