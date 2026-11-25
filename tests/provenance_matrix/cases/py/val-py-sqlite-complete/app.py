import sqlite3

def read():
    value = sqlite3.complete_statement("select 1")
    return value
