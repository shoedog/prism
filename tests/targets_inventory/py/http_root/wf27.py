import requests
from requests import get as fetch_http
requests.get = replacement
def fetch(u):
    return fetch_http(u)
