def read(paths):
    class Files:
        open = None
        handles = [open(p) for p in paths]
