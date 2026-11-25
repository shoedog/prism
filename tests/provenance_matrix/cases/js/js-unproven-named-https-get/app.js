import { get as read } from "node:https";

function request(url) {
  const requestHandle = read(url, () => {});
  return requestHandle;
}
