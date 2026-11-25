import requests
from compat import *


def fetch(u):
    return requests.get(u)
