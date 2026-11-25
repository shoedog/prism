from django.http import HttpResponse
from django.urls import path, re_path


def report(request, **params):
    return HttpResponse("ok")


def audited(view):
    def guarded(request, **params):
        return view(request, **params)

    return guarded


urlpatterns = [path("reports/", audited(report))]
