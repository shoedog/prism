from .base import Base
class Child(Base):
    def __init__(self, other):
        self.client = other
