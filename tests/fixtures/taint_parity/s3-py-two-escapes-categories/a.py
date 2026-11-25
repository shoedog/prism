import html
import markupsafe
import os
from django.utils.safestring import mark_safe

def entry():
    t = acquire()  # @SRC
    u = acquire()
    first = html.escape(s=t)
    second = markupsafe.escape(u)
    mark_safe(first); os.system(second)  # @SINK
