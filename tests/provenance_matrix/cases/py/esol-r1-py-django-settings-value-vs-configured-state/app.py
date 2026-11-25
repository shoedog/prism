from django.conf import settings as site_settings

def load():
    use_tz = site_settings.USE_TZ
    ready = site_settings.configured
    return use_tz, ready
