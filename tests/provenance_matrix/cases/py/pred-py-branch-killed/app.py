def choose(flag):
    value = input()
    if flag:
        value = "fixed"
    else:
        value = "safe"
    return value
