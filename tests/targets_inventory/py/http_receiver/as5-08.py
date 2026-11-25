from requests import Session
def fetch(u):
    return (client := Session()).get(u)
