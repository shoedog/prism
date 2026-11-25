const express = require("express");
const app = express();
app.get("/search", (req, res) => {
  const term = req.query.term;
  return term;
});
