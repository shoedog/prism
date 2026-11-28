from configparser import ConfigParser
class C(dict):
    def __init__(self, path: ConfigParser):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.read('settings.ini')
