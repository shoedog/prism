import json
from json import loads
json.loads = lambda text: {}
def parse(text):
    return loads(text)
