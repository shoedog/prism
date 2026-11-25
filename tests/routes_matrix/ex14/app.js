const express = require('express');
const app = express();

function named(req, res) { res.sendStatus(204); }

const register = (app) => {
  app.get('/x', named);
};

register(app);

module.exports = app;
