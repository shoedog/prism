const express = require("express");
const app = express();

function handler(req, res) {}

const x = {
  get(path, cb) {},
};

x.get("/y", handler);
