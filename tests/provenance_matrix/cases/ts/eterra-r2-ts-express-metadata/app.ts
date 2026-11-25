import express from "express";
const app = express();
app.get("/", (req, res) => {
  const route = req.route;
  res.send(route);
});
