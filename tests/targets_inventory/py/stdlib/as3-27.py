import os
def read(key, record):
    try:
        return os.getenv(key)
    finally:
        record(os.getenv(key))
