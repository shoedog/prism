from django.conf import settings
from django.urls import include, path

urlpatterns = [path("reports/", include(settings.REPORTS_URLCONF))]
