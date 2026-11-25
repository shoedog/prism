from dataclasses import dataclass, field
from requests import Session
@dataclass
class Client:
    session: Session = field(default_factory=Session)
    def fetch(self, u):
        return self.session.get(u)
