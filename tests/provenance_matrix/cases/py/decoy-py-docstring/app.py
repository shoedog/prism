def show():
    """request.args.get("mode") describes a different function."""
    return 7

def run():
    state = show()
    return state
