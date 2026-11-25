const express = require('express');
const app = express();

class Shelf {
  list(req, res) { res.json(this.items); }
}

const shelf = new Shelf();
shelf.items = ['notebooks'];
app.get('/shelf', shelf.list.bind(shelf));

module.exports = app;
