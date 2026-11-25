def parse(text):
    try:
        from json import loads, missing_member
    except ImportError:
        return loads(text)
