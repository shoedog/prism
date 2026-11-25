from django.urls import include, path

urlpatterns = [path("analytics/", include("reports", namespace="support_reports"))]
