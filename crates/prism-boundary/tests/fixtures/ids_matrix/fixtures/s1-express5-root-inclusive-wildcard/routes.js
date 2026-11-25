// Express 5.1.0
const express = require("express");
const app = express();

app.post("/{*waxTrail}", (req, res) => res.send("wax"));

module.exports = app;
