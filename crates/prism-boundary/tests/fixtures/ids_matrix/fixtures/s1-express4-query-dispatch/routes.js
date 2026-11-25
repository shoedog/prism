// Express 4.21.2
const express = require("express");
const app = express();

app.post("/fletch",
  (req, res, next) => req.query.cut === "round" ? next() : next("route"),
  (req, res) => res.send("wax"));
app.post("/fletch", (req, res) => res.send("pith"));

module.exports = app;
