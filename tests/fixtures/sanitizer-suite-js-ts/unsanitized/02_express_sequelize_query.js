import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
});

