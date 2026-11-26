import os

class Context:
    def __init__(self, value):
        self.value = value
    def __enter__(self):
        return self.value
    def __exit__(self, *exc):
        return False

def use(*, context):
    os.system(context)  # @SINK

def entry():
    t = acquire()  # @SRC
    with (
        Context(t) as context,
        Context(context) as second
    ):
        use(context=second)
