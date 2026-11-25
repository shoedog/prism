from b import run as go
def entry():
    t = acquire() # @SRC
    go(t)
