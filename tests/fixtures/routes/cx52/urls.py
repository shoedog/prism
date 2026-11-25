from django.http import HttpResponse
from django.urls import path, re_path


def report(request, **params):
    return HttpResponse("ok")


urlpatterns = [re_path(r"^reports/(?P<slug>[a-z-]+)/$", report)]
