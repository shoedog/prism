from . import requests

def load():
    result = requests.get("catalog")
    return result
