def outer(open):
    def read(path):
        global open
        return open(path)
    return read
