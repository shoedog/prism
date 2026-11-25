import sqlite3

def read():
    connection = sqlite3.connect(":memory:")
    result = connection.close()
    return result
