import sqlite3

def load(path):
    connection = sqlite3.connect(path)
    try:
        with connection.blobopen("catalog", "thumbnail", 1, readonly=True) as blob:
            image = blob.read(16)
            position = blob.tell()
            return image, position
    finally:
        connection.close()
