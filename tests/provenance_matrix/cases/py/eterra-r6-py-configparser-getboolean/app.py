import configparser
def read():
    parser = configparser.ConfigParser()
    parser.read('app.ini')
    enabled = parser.getboolean('app', 'enabled')
    return enabled
