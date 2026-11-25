import yaml

def read():
    value = yaml.safe_load("mode: quiet")
    return value
