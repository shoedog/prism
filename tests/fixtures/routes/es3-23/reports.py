from django.http import HttpResponse
from django.urls import path


def archived(request):
    return HttpResponse('archived')


def current(request):
    return HttpResponse('current')


urlpatterns = [path('summary/', archived)]
next_patterns = [path('summary/', current)]
