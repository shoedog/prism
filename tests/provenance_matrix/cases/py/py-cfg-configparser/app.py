from configparser import ConfigParser

def load():
    parser = ConfigParser()
    parser.read("app.ini")
    mode = parser.get("config", "request-id")
    return mode
