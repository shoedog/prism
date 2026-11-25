const express = require('express');
const app = express();
function list(req, res) { res.end('list'); }
function create(req, res) { res.end('create'); }
const resource = app.route('/items');
resource.get(list);
resource.post(create);
module.exports = app;
