from typing import Optional

import requests


def f(s: Optional[requests.Session], u):
    return s.get(u)
