import requests
def fetch(u):
    return (lambda reply=requests.get(u): reply)()
