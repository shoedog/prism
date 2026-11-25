import requests
def f(sessions: dict[str, requests.Session], key):
    return sessions.get(key)
