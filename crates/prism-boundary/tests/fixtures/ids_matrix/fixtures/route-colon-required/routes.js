const express = require("express");
const router = express.Router();

function route(entries) {
  for (const { method, path, handler } of entries) {
    router[method.toLowerCase()](path, (request, response) => response.send(handler()));
  }
}

route([
  { method: "GET", path: "/mugs/:size", handler: () => "1" },
]);
