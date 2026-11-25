import os
mode = "fixed"

def activate():
    global mode
    mode = os.getenv("MODE")
    return mode
