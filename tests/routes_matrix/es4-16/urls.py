from django.http import HttpResponse
from django.urls import path

def current(request):
    return HttpResponse('current')

def legacy(request):
    return HttpResponse('legacy')

urlpatterns = [path('current/', current), path('current/', legacy)]
