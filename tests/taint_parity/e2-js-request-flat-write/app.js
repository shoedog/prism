import express from "express";
const app = express();
app.get("/", (req, res) => { // @SRC
  observe(req.query.term); node.innerHTML = req.query.term; // @SINK
});
