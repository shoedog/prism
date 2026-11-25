import os
def run(flag):
    value = os.getenv("MODE")
    if flag:
        value = input()
        return value
    print(value)
