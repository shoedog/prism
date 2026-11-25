import requests
def f(dispatch, u):
    return (dispatch[
            0](), requests.get(u))
