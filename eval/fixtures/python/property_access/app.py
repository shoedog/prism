class Response:
    @property
    def text(self):
        return self._text


def f(r):
    return r.text
