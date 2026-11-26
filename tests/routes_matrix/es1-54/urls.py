from django.conf.urls.i18n import i18n_patterns
from django.http import HttpResponse
from django.urls import path

def help_page(request):
    return HttpResponse("help")

urlpatterns = i18n_patterns(path("help/", help_page), prefix_default_language=False)
