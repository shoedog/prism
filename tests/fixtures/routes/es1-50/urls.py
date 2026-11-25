from django.urls import include, path
from features import reports as reporting
from features.reports import urlpatterns as report_patterns

urlpatterns = [
    path("ops/", include(reporting)),
    path("support/", include(report_patterns)),
]
