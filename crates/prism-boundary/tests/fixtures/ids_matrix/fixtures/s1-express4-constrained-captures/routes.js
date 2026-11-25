// Express 4.21.2
const express = require("express");
const app = express();

app.post("/fletch/:stamp([0-9]+)", (req, res) => res.send("wax"));
app.post("/fletch/:stamp([a-z]+)", (req, res) => res.send("pith"));

module.exports = app;
