import requests as __class__
class Index(dict):
    @classmethod
    def fetch(cls, data, key):
        return __class__.get(data, key)
