import requests
def fetch(sink, u):
    return sink(
        tag=requests.get(u), *requests.post(u),
    )
