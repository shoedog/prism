class Response:
    @property
    def text(self):
        return self._text

    @text.setter
    def text(self, value):
        self._text = value


def f(r):
    r.text = "v"
