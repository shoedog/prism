from django.http import HttpResponse
from django.urls import path, re_path


def report(request, **params):
    return HttpResponse("ok")


urlpatterns = [path("reports/<int:report_id>/", report)]
