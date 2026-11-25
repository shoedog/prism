import sqlite3

def load():
    connection = sqlite3.connect(":memory:")
    cursor = connection.cursor()
    executed = cursor.execute("select 1")
    row = executed.fetchone()
    return row
