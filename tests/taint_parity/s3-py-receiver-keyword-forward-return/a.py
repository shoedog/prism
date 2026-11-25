import os

class Manager:
    def webhook(self, request, *, channel=None):
        return channel

def get_manager():
    return Manager()

def forward(request, channel):
    return get_manager().webhook(request, channel=channel)

def entry(request):
    t = acquire()  # @SRC
    result = forward(request, t)
    os.system(result)  # @SINK
