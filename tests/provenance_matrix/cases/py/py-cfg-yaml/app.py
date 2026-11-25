import yaml

def load():
    with open("settings.yaml") as stream:
        settings = yaml.safe_load(stream)
    return settings
