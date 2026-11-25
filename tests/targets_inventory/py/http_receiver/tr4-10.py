import requests
def f(value):
    s = requests.Session()
    match value:
        case {"client": s}: pass
    return s.get("u")
