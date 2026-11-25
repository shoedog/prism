from django.conf import settings
from django.core.wsgi import get_wsgi_application

settings.configure(
    ROOT_URLCONF="urls",
    MIDDLEWARE=["django.middleware.locale.LocaleMiddleware"],
    USE_I18N=True,
    LANGUAGE_CODE="en",
    LANGUAGES=[("en", "English"), ("fr", "French")],
)

application = get_wsgi_application()
