# Pinned occurrence rows of the Python alias-twin classifier (the slice-0 oracle's tokens).
def t(o, q):
    u = o            # EXPECT u:binding-write:Y-01
    u = q            # EXPECT u:binding-write:Y-01
    u.f = 1          # EXPECT u.f:storage-write:Y-03
    u[0] = 2         # EXPECT u[]:storage-write:Y-03
    u += q           # EXPECT u:unsupported:Y-02
    w = o            # EXPECT w:binding-write:Y-01
    with open(q) as w:    # EXPECT w:binding-write:Y-01
        pass
    a, b = o, q      # EXPECT a:binding-write:Y-01 b:binding-write:Y-01
    c = d = o        # EXPECT c:binding-write:Y-01 d:binding-write:Y-01
    for k in q:
        pass
    if (n := o):     # EXPECT n:binding-write:Y-01
        pass
    try:
        pass
    except Exception as err:   # EXPECT err:binding-write:Y-01
        pass


def adversarial(o, q, xs):
    u = o            # EXPECT u:binding-write:Y-01
    for u.x in xs:
        pass
    ys = [z for z in xs]   # EXPECT ys:binding-write:Y-01
