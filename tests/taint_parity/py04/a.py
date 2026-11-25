import html
from django.utils.safestring import mark_safe

def main():
    t = input()  # @SRC
    s = html.escape(t)
    return mark_safe(s)  # @SINK

main()
