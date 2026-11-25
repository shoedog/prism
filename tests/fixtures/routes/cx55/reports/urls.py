from django.http import HttpResponse
from django.urls import path


def report(request, **params):
    return HttpResponse("ok")


urlpatterns = [path("reports/", report)]
