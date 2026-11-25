import requests
def f():
    session = requests.Session()
    return [session.get("element")
            for session in session.get("iterable")]
