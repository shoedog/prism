import os
def clean(value):
    return value.strip()
def run():
    raw = os.getenv('MODE')
    value = clean(raw)
    return value # final
