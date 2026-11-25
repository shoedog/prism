export function deliver(method: string, path: string, payload?: unknown) {
  return Promise.resolve({ data: { method, path, payload } });
}
export const mintAlderfenCourier = {
  create({ baseURL }: { baseURL: string }) {
    const wire = (method: string, path: string, payload?: unknown) =>
      deliver(method, baseURL.replace(/\/$/, '') + '/' + path.replace(/^\/+/, ''), payload);
    let nextTicket = 0;
    const interceptor = {
      use: (_success: (value: any) => any, _failure?: (error: any) => unknown) => ++nextTicket,
      eject: (_ticket: number) => undefined,
    };
    return {
      get<T = unknown>(path: string, options?: unknown) { return wire('GET', path, options); },
      post(path: string, body?: unknown) { return wire('POST', path, body); },
      put<T = unknown>(path: string, body?: unknown, options?: unknown) {
        return wire('PUT', path, { body, options });
      },
      delete(path: string, options?: unknown) { return wire('DELETE', path, options); },
      interceptors: { request: interceptor, response: interceptor },
    };
  },
};
