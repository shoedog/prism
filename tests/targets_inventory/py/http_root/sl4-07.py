import requests
def f(routes, u):
    return routes[requests.get(u)]()
