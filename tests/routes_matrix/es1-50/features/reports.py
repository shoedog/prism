from django.http import HttpResponse
from django.urls import path

def report(request):
    return HttpResponse("report")

urlpatterns = [path("daily/", report)]
