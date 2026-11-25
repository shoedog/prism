class ForeignModule:
    exports = "fixed"

class Display:
    def caption(self):
        value = self.data.x
        return value

def read(fd):
    return "fixed"

def show():
    module = ForeignModule()
    exported = module.exports
    fd = len(())
    x = read(fd)
    return exported, x, read(fd)
