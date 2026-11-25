import httpx
import requests
def fetch(flag, u):
    return (requests.get if flag else httpx.get)(u)
