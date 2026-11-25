import html
from django.utils.safestring import mark_safe

def transform(x):
    y = html.escape(x)
    return y

def entry(flag):
    t = acquire()  # @SRC
    u = acquire()
    clean = transform(
        t
    )
    value = clean if flag else u
    return mark_safe(value)  # @SINK
