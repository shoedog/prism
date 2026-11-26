from django.http import HttpResponse
from django.urls import path

def hours(request):
    return HttpResponse('hours')

urlpatterns = [path('hours/', hours)]
