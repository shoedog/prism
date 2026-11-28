export {};
export function createApplication() {
  var app = function (req: number) { return req; };
  var proto = {
    request: { app: { configurable: true, enumerable: true, writable: true, value: app } },
    response: { app: { configurable: true, enumerable: true, writable: true, value: app } },
  };
  return proto;
}
export function render(this: { engines: number }) {
  var engines = this.engines;
  var renderOptions = { engines: engines };
  return renderOptions;
}
export function request(url: string, method: string) {
  return { url: url, method: method };
}
