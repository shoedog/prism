def show():
    literal = str("request.args.get")
    decorated = f"request.args.get={7}"
    return literal, decorated
