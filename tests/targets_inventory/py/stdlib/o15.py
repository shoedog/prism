from os import environ


def home():
    return environ.get("HOME")
