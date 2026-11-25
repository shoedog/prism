import requests
def f(u):
    try:
        raise ExceptionGroup("errors", [ValueError()])
    except* ValueError as requests:
        pass
    return requests.get(u)
