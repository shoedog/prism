import os
def run(stop):
    value = os.getenv('MODE')
    if stop:
        value = input()
        return
    print(value)
