import os
def read():
    value = os.getenv('MODE')
    try:
        pass
    finally:
        value = 'fixed'
    return value
