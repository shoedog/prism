from django.conf import settings
from django.http import HttpResponse
from django.urls import path


def current(request):
    return HttpResponse("current bulletin")


def preview(request):
    return HttpResponse("preview bulletin")


urlpatterns = [path("bulletin/", current)]
if settings.DEBUG:
    urlpatterns += [path("bulletin/preview/", preview)]
