import os

class Query:
    def using(self, name):
        return self
    def get(self, **kw):
        x = kw["id"]
        os.system(x)  # @SINK

def entry(use_id):
    t = acquire()  # @SRC
    key = "id" if use_id else "uuid"
    Query().using("fixture").get(**{key: t})
