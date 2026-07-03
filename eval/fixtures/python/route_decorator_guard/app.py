from flask import Flask

app = Flask(__name__)


class Foo:
    @property
    def value(self):
        return 1
