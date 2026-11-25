import configparser as ini

def load():
    parser = ini.ConfigParser()
    parser.read("catalog.ini")
    section = parser["display"]
    title = section["title"]
    return title
