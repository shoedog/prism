import requests
def fetch(rows, u):
    [[(requests := client) for client in row] for row in rows]
    return requests.get(u)
