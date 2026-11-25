from django.http import HttpResponse
from django.urls import path
def report(request):
    return HttpResponse("ok")
urlpatterns = [path("daily/", report)]
