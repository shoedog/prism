from contextlib import contextmanager

import requests


@contextmanager
def make() -> requests.Session:
    return requests.Session()


def fetch(u):
    s = make()
    return s.get(u)
