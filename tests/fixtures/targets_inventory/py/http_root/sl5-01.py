import requests
def f(mapping, u):
    mapping[requests.get(u)] = requests.post(u)
