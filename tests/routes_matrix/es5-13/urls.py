from django.http import HttpResponse
from django.urls import re_path


def exact(request):
    return HttpResponse("exact")


def fallback(request):
    return HttpResponse("fallback")


urlpatterns = [
    re_path(r"^status/$", exact),
    re_path(r"^status/", fallback),
]
