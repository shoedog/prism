class Args:
    def get(self, key):
        return "fixed"

class LocalRequest:
    args = Args()

request = LocalRequest()
