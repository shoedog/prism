def read(consume, path):
    out: open(path) = consume(open(path), open(path))
