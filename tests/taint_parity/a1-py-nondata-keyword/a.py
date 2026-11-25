import html
import os

def entry():
    flag = acquire()  # @SRC
    out = html.escape(s="plain", quote=flag)
    os.system(out)  # @SINK
