from django.http import HttpResponse
from django.urls import path


def overview(request):
    return HttpResponse('overview')


def hours(request):
    return HttpResponse('hours')


urlpatterns = [path('', overview), path('hours/', hours)]
