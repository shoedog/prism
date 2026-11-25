import requests
class C:
    def __init__(self):
        self.__session = requests.Session()
    def reset(self, value):
        setattr(self, "__session", value)
    def f(self, u):
        return self.__session.get(u)
