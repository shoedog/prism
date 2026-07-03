class Response:
    @property
    def text(self):
        return self._text

    def dump(self):
        return self.text
