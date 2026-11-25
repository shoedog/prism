import os
def entry(record):
    t = acquire() # @SRC
    return getattr(record, t) # @SINK
