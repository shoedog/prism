import io
def read(record, path):
    return record(open(path), io.open(path))
