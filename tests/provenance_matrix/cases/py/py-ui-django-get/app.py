from django.http import HttpRequest

def search(request: HttpRequest):
    term = request.GET["term"]
    return term
