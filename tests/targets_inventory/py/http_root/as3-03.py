def fetch(u):
    return __import__("requests").get(u)
