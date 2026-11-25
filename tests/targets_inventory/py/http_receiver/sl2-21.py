import requests as http
def f(u):
    requests = http.Session()
    return requests.get(u)
