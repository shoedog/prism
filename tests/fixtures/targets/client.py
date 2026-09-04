class Args:
    def get(self, name):
        return name


class Request:
    args = Args()


request = Request()


def fetch(user):
    if not user: return None
    query = request.args.get("q")
    handle = open("x")
    return query


def serialize_x(value):
    return str(value)


def deserialize_x(value):
    return value
