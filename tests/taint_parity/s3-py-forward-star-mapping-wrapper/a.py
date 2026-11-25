import os

def serve(*args, path):
    return path

def wrapper(*args, **kwargs):
    return serve(*args, **kwargs)

def entry():
    t = acquire()  # @SRC
    result = wrapper("context", path=t)
    os.system(result)  # @SINK
