import requests

def labels():
    reply = requests.get("https://catalog.example/edition")
    data = reply.text
    return data
