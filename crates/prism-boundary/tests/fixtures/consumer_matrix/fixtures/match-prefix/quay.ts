export function recordSable(method: string, path: string, body?: unknown) {
  return Promise.resolve({ data: { method, path, body } });
}

const mount = "/api";
const strip = "/legacy";
function sendSable(method: string, path: string, body?: unknown) {
  const suffix = strip && path.startsWith(strip) ? path.slice(strip.length) : path;
  return recordSable(method, mount ? mount + '/' + suffix.replace(/^\/+/, '') : path, body);
}

const get = <T = unknown>(path: string, options?: unknown) => sendSable('GET', path, options);
const post = (path: string, body?: unknown) => sendSable('POST', path, body);
const put = (path: string, body?: unknown) => sendSable('PUT', path, body);
export default {
  get, post, put,
  delete: (path: string, options?: unknown) => sendSable('DELETE', path, options),
  patch: (path: string, body?: unknown) => sendSable('PATCH', path, body),
  head: (path: string) => sendSable('HEAD', path),
  options: (path: string) => sendSable('OPTIONS', path),
  instance: { get, post, put },

};
