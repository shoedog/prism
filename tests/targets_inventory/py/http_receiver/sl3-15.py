from dataclasses import dataclass
import requests
@dataclass
class Client:
    session: object = None
    def __post_init__(self):
        self.session = requests.Session()
    def f(self, u):
        return self.session.get(u)
