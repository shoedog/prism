from django.http import HttpRequest

def read(request: HttpRequest):
    name, count = request.GET["q"], 7
    return name, count
