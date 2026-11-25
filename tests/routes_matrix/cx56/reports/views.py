from django.http import HttpResponse


def report(request, **params):
    return HttpResponse("ok")
