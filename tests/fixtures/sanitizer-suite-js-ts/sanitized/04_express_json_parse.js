import express from "express";

const app = express();

app.post("/json", (req, res) => {
  const payload = req.body.payload;
  const parsed = JSON.parse(payload);
  return res.json(parsed);
});

