import express, { Request } from "express";
const app = express();

app.post("/preview", (req, res) => {
  const payload = (req as Request).body;
  return payload;
});
