import sqlite3

def bind():
    connection = sqlite3.connect(":memory:")

    def read():
        rows = connection.execute("select 1")
        return rows

    return read
