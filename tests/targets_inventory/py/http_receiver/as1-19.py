from requests import Session
def fetch(typed: Session, use, u):
    local = Session()
    return use(local.get(u), typed.get(u))
