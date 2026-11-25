from django.urls import include, path
import reports

urlpatterns = [
    path('snapshot/', include((reports.urlpatterns, 'snapshot'))),
    path('live/', include(reports)),
]

reports.urlpatterns = reports.next_patterns
