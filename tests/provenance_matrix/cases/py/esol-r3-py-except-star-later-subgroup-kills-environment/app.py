import os

def preview():
    label = "initial"
    try:
        raise ExceptionGroup("preview", [ValueError(), TypeError()])
    except* ValueError:
        label = os.getenv("PREVIEW_LABEL")
    except* TypeError:
        label = "fallback"
    return label
