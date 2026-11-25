import os

from django.conf import settings
from django.core.wsgi import get_wsgi_application

settings.configure(
    ROOT_URLCONF="urls",
    REPORTS_URLCONF="preview" if os.getenv("REPORTS_PREVIEW") == "1" else "published",
)

application = get_wsgi_application()
