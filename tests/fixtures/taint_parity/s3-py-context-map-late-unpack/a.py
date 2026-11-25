import os

def render(template, context):
    x = context["url"]
    os.system(x)  # @SINK

def entry():
    t = acquire()  # @SRC
    return render(
        "home",
        {"url": t, **{"url": "fixed"}, "name": t}
    )
