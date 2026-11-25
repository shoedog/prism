def isolated():
    t = acquire()  # @SRC
    observe(t)

def lookup(request):
    value = request.GET["key"]
    return getattr(record, value)  # @SINK
