import sqlite3

def load():
    connection = sqlite3.connect(":memory:")
    cursor = connection.cursor()
    cursor.execute("select 1")
    row = cursor.fetchone()
    return row
