from django.conf import settings
from django.core.wsgi import get_wsgi_application

settings.configure(
    ROOT_URLCONF="urls",
    MIDDLEWARE=["office.OfficeDirectory"],
)

application = get_wsgi_application()
