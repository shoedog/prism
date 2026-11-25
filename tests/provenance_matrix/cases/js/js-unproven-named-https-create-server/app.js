import { createServer as read } from "node:https";

function prepare() {
  const server = read();
  return server;
}
