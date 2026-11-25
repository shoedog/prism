def outer():
    open = None
    def inner():
        return open("file")
    return inner
