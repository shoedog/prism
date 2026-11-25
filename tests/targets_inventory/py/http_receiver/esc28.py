import requests
from requests import Session as C
requests.Session = dict
def fetch(u):
    s = C()
    return s.get(u)
