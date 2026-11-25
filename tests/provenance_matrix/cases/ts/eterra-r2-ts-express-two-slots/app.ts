import express from "express";
const app = express();
app.get("/", (req, res) => {
  const [term, label] = [req.query.q, "fixed"];
  res.send([term, label]);
});
