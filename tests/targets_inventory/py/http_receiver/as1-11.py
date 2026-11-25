from requests import Session
def send(req):
    transmit = Session().send
    return transmit(req)
