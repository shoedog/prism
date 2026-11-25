import os
def run():
    values = [os.getenv('MODE') for os in [Fake()]]
    return values
class Fake:
    def getenv(self, key):
        return 'local' 
