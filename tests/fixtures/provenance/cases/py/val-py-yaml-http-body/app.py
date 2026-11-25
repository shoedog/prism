import requests
import yaml

def read(url):
    response = requests.get(url)
    value = yaml.safe_load(response.text)
    return value
