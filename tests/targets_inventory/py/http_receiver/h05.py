import requests


def send(req):
    s = requests.Session()
    return s.send(req)
