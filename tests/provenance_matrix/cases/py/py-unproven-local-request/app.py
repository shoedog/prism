class Args:
    def get(self, key):
        return key

class LocalRequest:
    args = Args()

request = LocalRequest()

def read():
    value = request.args.get("mode")
    return value
