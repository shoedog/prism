from django.http import HttpResponse
from django.urls import path, re_path


def report(request, **params):
    return HttpResponse("ok")


views = {"report": report}


urlpatterns = [path("reports/", views["report"])]
