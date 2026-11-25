import requests
from shim import *
def fetch(u):
    return requests.get(u)
