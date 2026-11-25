import requests

PACKAGE_INDEX_LINK = 'https://example.test/index.json'


def _fetch_updates(env):
    file = env.config.version_info_file
    data = _read_data_error_free(file)

    response = requests.get(PACKAGE_INDEX_LINK, verify=False)
    response.raise_for_status()
    return response.json()
