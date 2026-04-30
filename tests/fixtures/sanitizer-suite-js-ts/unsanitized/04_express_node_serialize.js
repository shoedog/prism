import express from "express";
import serialize from "node-serialize";

const app = express();

app.post("/payload", (req, res) => {
  const payload = req.body.payload;
  return serialize.unserialize(payload);
});

