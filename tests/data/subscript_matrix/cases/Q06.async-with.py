def case():
    t = src()
    try:
        run(t).send(None)
    except StopIteration:
        pass

class Manager:
    async def __aenter__(self):
        return self

    async def __aexit__(self, exc, value, tb):
        return False

async def run(t):
    d = {}
    d["a"] = t
    async with Manager():
        spare = 0
    sink(d)
