from django.conf import settings
from django.core.wsgi import get_wsgi_application

settings.configure(
    ROOT_URLCONF="urls",
    DEBUG=False,
    ALLOWED_HOSTS=["testserver"],
    APPEND_SLASH=True,
    MIDDLEWARE=["django.middleware.common.CommonMiddleware"],
)

application = get_wsgi_application()
