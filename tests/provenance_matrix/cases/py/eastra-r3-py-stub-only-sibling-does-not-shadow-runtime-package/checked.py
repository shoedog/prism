from typing import TYPE_CHECKING
import requests
if TYPE_CHECKING:
    from unittest.mock import Mock
    requests.get = Mock()

def labels():
    reply = requests.get("https://catalog.example/edition")
    data = reply.text
    return data
