def outer(settings):
    def inner():
        return settings.name
    return inner
