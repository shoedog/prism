import requests
class C:
    @property
    def value(self):
        return self._value
    @value.setter
    def value(self, u):
        self._value = requests.get(u)
