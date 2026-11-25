import requests


def build_requests_session(verify, ciphers=None) -> requests.Session:
    requests_session = requests.Session()
    http_adapter = make_adapter()
    https_adapter = make_adapter(ciphers=ciphers, verify=verify)
    requests_session.mount('http://', http_adapter)
    requests_session.mount('https://', https_adapter)
    for plugin_cls in get_transport_plugins():
        transport_plugin = plugin_cls()
        requests_session.mount(
            prefix=transport_plugin.prefix,
            adapter=transport_plugin.get_adapter(),
        )
    return requests_session


def collect_messages(args, httpie_session, send_kwargs):
    requests_session = build_requests_session(verify=args.verify)
    if httpie_session:
        requests_session.cookies = httpie_session.cookies
    prepared_request = requests_session.prepare_request(args.request)
    while prepared_request:
        yield prepared_request
        if not args.offline:
            response = requests_session.send(
                request=prepared_request,
                **send_kwargs,
            )
            prepared_request = None
