import requests
from requests import Session as Class
def fetch(u, replacement):
    Class.get = replacement
    session = requests.Session()
    return session.get(u)
