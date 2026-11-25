import {load} from "js-yaml";
import express from "express";
const app = express();
app.get("/", (req, res) => {
  return load( // @SINK
    req.body.payload
  );
});

function parityAnchor() {
  const t = acquire(); // @SRC
}
