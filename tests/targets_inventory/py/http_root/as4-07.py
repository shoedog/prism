import requests
def fetch(factory, u):
    return factory(requests.get(u))(
                                  )
