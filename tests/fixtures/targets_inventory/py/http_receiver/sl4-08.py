from requests import Session
def f(u):
    with (Session() as session,
          session.get(u)):
        pass
