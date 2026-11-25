// Express 5.1.0
const express = require("express");
const app = express();

app.post('/fletch/:"wax-label"', (req, res) => res.send(req.params["wax-label"]));

module.exports = app;
