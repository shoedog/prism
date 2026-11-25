type RequestConfig = string;
export function read() {
  const mode: RequestConfig = process.env.MODE;
  const copy: RequestConfig = mode;
  return copy;
}
