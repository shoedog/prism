from requests import Session
def fetch(u, s: Session = Session()):
    return s.get(u)
fetch("u", {})
