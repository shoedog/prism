import requests


def fetch(log, u):
    log.info('x'); r = requests.get(u)
