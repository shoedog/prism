import requests
requests.get = replacement
from requests import get as fetch_http
def fetch(u):
    return fetch_http(u)
