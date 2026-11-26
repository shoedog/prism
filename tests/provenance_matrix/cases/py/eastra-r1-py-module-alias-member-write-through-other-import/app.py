import requests as first
import requests as second
def replacement(url):
    return "sample"
first.get = replacement
def preview():
    label = second.get("https://example.invalid/edition")
    return label
