import os
def run(flag):
    if flag:
        value = input()
    else:
        value = os.getenv("MODE")
    return value
