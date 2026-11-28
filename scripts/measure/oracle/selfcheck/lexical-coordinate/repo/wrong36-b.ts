export {};
declare function use(v: unknown): void;
export function sendfile(filename: string, options: unknown, done: () => void) {
  var opts: unknown = options;
  opts = filename;
  use(filename);
  use(options);
  var finished = done;
  var value = filename;
  use(value);
  use(finished);
}
