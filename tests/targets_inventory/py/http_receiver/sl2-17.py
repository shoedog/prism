import requests
class C:
    def __init__(self):
        self.__session = requests.Session()
    def reset(self, value):
        self._C__session = value
    def f(self, u):
        return self.__session.get(u)
