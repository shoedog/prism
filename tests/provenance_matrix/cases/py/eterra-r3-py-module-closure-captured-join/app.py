import os
mode = 'default'
def read():
    return mode
mode = os.getenv('MODE')
