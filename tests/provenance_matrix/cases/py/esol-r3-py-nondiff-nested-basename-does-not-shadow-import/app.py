import requests

def load():
    reply = requests.get("https://example.invalid/catalog")
    body = reply.text
    return body
