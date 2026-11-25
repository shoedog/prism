// Express 5.1.0
const express = require("express");
const app = express();

const spindle = express.Router();
spindle.post("/proof", (req, res) => res.send("wax"));
app.use(["/amber", "/ochre"], spindle);

module.exports = app;
