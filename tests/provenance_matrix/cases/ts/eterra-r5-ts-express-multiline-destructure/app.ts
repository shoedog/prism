import express from "express";
const app = express();
app.get("/", (req, res) => {
  const {
    q: term
  } = req.query;
  res.send(term);
});
